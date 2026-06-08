//! Pre-allocated real/complex scratch buffers and the helpers that copy/convert
//! between them — the allocation-free scratch management every FFT-based DSP
//! stage shares.
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

use crate::Float;

pub enum ComplexComponent {
    Re,
    Im,
}

pub fn new_real_buffer<T: Float>(size: usize) -> Vec<T> {
    vec![T::zero(); size]
}

pub fn new_complex_buffer<T: Float>(size: usize) -> Vec<Complex<T>> {
    vec![Complex::zero(); size]
}

pub fn copy_real_to_complex<T: Float>(
    input: &[T],
    output: &mut [Complex<T>],
    component: ComplexComponent,
) {
    assert!(input.len() <= output.len());
    match component {
        ComplexComponent::Re => input.iter().zip(output.iter_mut()).for_each(|(i, o)| {
            o.re = *i;
            o.im = T::zero();
        }),
        ComplexComponent::Im => input.iter().zip(output.iter_mut()).for_each(|(i, o)| {
            o.im = *i;
            o.re = T::zero();
        }),
    }
    output[input.len()..]
        .iter_mut()
        .for_each(|o| *o = Complex::zero())
}

pub fn copy_complex_to_real<T: Float>(
    input: &[Complex<T>],
    output: &mut [T],
    component: ComplexComponent,
) {
    assert!(input.len() <= output.len());
    match component {
        ComplexComponent::Re => input
            .iter()
            .map(|c| c.re)
            .zip(output.iter_mut())
            .for_each(|(i, o)| *o = i),
        ComplexComponent::Im => input
            .iter()
            .map(|c| c.im)
            .zip(output.iter_mut())
            .for_each(|(i, o)| *o = i),
    }

    output[input.len()..]
        .iter_mut()
        .for_each(|o| *o = T::zero());
}

/// Computes |x|^2 for each complex value x in `arr`. This function
/// modifies `arr` in place and leaves the complex component zero.
pub fn modulus_squared<T: Float>(arr: &mut [Complex<T>]) {
    for s in arr.iter_mut() {
        s.re = s.re * s.re + s.im * s.im;
        s.im = T::zero();
    }
}

/// Compute the sum of the square of each element of `arr`.
pub fn square_sum<T>(arr: &[T]) -> T
where
    T: Float + std::iter::Sum,
{
    arr.iter().map(|&s| s * s).sum::<T>()
}

/// The number of complex scratch buffers every detector's hot path needs at once
/// (`windowed_autocorrelation` borrows three simultaneously; `autocorrelation` borrows two).
const COMPLEX_BUFFER_COUNT: usize = 3;
/// The number of real scratch buffers needed at once (`normalized_square_difference`'s
/// `m_of_tau` scratch). The detector's *result* buffer lives outside the pool, so a single
/// real scratch buffer is enough.
const REAL_BUFFER_COUNT: usize = 1;

#[derive(Debug)]
/// A pool of pre-allocated real/complex scratch buffers, owned outright so the pool (and any
/// detector holding one) is `Send`. All buffers are allocated up front by [`BufferPool::new`]
/// and never freed; the hot path borrows disjoint slices out of the pool instead of allocating,
/// which keeps `get_pitch` allocation-free (the crate's WASM / real-time contract).
///
/// Buffers are handed out as disjoint mutable slices (via [`BufferPool::complex_pair`],
/// [`BufferPool::complex_triple`], and [`BufferPool::real`]) rather than reference-counted cells,
/// so there is no interior mutability and nothing to make the pool `!Send`.
///
/// ```rust
///  use prism::buffer::BufferPool;
///
///  let mut buffers = BufferPool::<f64>::new(3);
///  let (a, b) = buffers.complex_pair();
///  a[0].re = 5.5;
///  b[0].re = 6.6;
///  // The two slices are distinct buffers, so both writes stick.
///  assert_eq!(a[0].re, 5.5);
///  assert_eq!(b[0].re, 6.6);
/// ```
pub struct BufferPool<T> {
    real_buffers: Vec<Vec<T>>,
    complex_buffers: Vec<Vec<Complex<T>>>,
    pub buffer_size: usize,
}

impl<T: Float> BufferPool<T> {
    /// Create a pool with every scratch buffer pre-allocated to `buffer_size`, ready for the
    /// detector hot path. Allocation happens once, here — never inside `get_pitch`.
    pub fn new(buffer_size: usize) -> Self {
        Self::new_for_fft(buffer_size, buffer_size)
    }

    /// Create a pool whose complex scratch buffers are large enough to double as rustfft in-place
    /// scratch.
    ///
    /// rustfft's `Fft::process_with_scratch` requires the scratch slice be at least
    /// `Fft::get_inplace_scratch_len()` long — a value that is **not** bounded by the transform
    /// length. For Bluestein-routed lengths (e.g. primes whose `len - 1` has a large prime factor,
    /// like 83, or prime powers like 169/289) it exceeds the FFT length, so a pool sized only to
    /// `buffer_size` would make `process_with_scratch` panic.
    ///
    /// Buffer roles (see [`complex_pair`](Self::complex_pair) / [`complex_triple`](Self::complex_triple)):
    /// the **first** complex buffer is only ever the in-place FFT *data* buffer, so it is kept at
    /// exactly `buffer_size` — rustfft requires the data length to be an exact multiple of the
    /// transform length, so it must not be widened. The remaining complex buffers are handed out as
    /// FFT *scratch* (the second one also as a secondary data buffer, where the caller slices it back
    /// down), so they are sized to `buffer_size.max(fft_scratch_len)`. `buffer_size` (the logical
    /// transform length) is preserved as the field used by length assertions; the real scratch never
    /// feeds an FFT, so it stays at `buffer_size`.
    pub fn new_for_fft(buffer_size: usize, fft_scratch_len: usize) -> Self {
        let scratch_size = buffer_size.max(fft_scratch_len);
        let complex_buffers = (0..COMPLEX_BUFFER_COUNT)
            .map(|i| new_complex_buffer::<T>(if i == 0 { buffer_size } else { scratch_size }))
            .collect();
        BufferPool {
            real_buffers: (0..REAL_BUFFER_COUNT)
                .map(|_| new_real_buffer::<T>(buffer_size))
                .collect(),
            complex_buffers,
            buffer_size,
        }
    }

    /// Two disjoint complex scratch buffers, each `buffer_size` long.
    pub fn complex_pair(&mut self) -> (&mut [Complex<T>], &mut [Complex<T>]) {
        let (first, rest) = self.complex_buffers.split_at_mut(1);
        (first[0].as_mut_slice(), rest[0].as_mut_slice())
    }

    /// Three disjoint complex scratch buffers, each `buffer_size` long.
    #[allow(clippy::type_complexity)]
    pub fn complex_triple(&mut self) -> (&mut [Complex<T>], &mut [Complex<T>], &mut [Complex<T>]) {
        let (first, rest) = self.complex_buffers.split_at_mut(1);
        let (second, third) = rest.split_at_mut(1);
        (
            first[0].as_mut_slice(),
            second[0].as_mut_slice(),
            third[0].as_mut_slice(),
        )
    }

    /// A single real scratch buffer, `buffer_size` long.
    pub fn real(&mut self) -> &mut [T] {
        self.real_buffers[0].as_mut_slice()
    }
}

#[test]
fn test_buffers() {
    let mut buffers = BufferPool::<f64>::new(3);

    // The three complex scratch buffers are distinct, so writes to each are independent.
    let (a, b, c) = buffers.complex_triple();
    a[0].re = 5.5;
    b[1].re = 6.6;
    c[2].re = 7.7;
    assert_eq!(a[0].re, 5.5);
    assert_eq!(b[1].re, 6.6);
    assert_eq!(c[2].re, 7.7);

    // The real scratch buffer is independent of the complex ones.
    let real = buffers.real();
    real[0] = 1.5;
    assert_eq!(real, &[1.5, 0., 0.]);
}
