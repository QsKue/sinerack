# Buffer

Source: `src/buffer.rs`

The allocation-free scratch management every FFT-based DSP stage shares: a `BufferPool<T>` plus small
real/complex copy/convert helpers. It was distilled out of reed, which now re-exports it as
`reed::utils::buffer::*`. Nothing here knows about a specific algorithm — it is pure buffer/numeric
plumbing.

## `BufferPool<T>`

The workspace's allocation-reuse mechanism. It owns a fixed set of pre-allocated real/complex scratch
buffers and lends them out as **disjoint mutable slices**: `complex_pair` (two), `complex_triple`
(three), and `real` (one). The buffers are allocated once (in `new` / `new_for_fft`) and reused on
every call, so a stage's hot path runs repeatedly — including in WASM and real-time callbacks —
without per-call allocation. There is no interior mutability (no `Rc`/`RefCell`), so the pool — and
every stage holding one — is `Send`.

## Helpers

- `new_real_buffer` / `new_complex_buffer` — zero-filled `Vec<T>` / `Vec<Complex<T>>`.
- `copy_real_to_complex` / `copy_complex_to_real` — with a `ComplexComponent::{Re, Im}` selector;
  both zero the tail past the copied input.
- `modulus_squared` — in-place `|x|²`, zeroing the imaginary part.
- `square_sum` — Σ xᵢ² (the power metric a detector's `power_threshold` gate uses). Carries the extra
  `std::iter::Sum` bound at this use site.

## Patterns to follow

- Keep this code generic over `T: Float` and free of algorithm-specific or musical concepts — stages
  decide *what* to do; `buffer` just executes the copy/convert/scratch mechanics.
- When a stage needs new shared scratch, get it from the `BufferPool` rather than allocating in the
  hot path.

## What should not be placed here

- Any domain concept (pitch, clarity, stretch ratio, denoise gain) or algorithm orchestration. This
  is the lowest-level shared plumbing; leaf logic lives in the leaves.

## Gotchas

- **`new_for_fft` scratch sizing.** `BufferPool::new` calls `new_for_fft(buffer_size, buffer_size)`.
  The `new_for_fft(buffer_size, fft_scratch_len)` constructor exists because rustfft's
  `Fft::process_with_scratch` needs a scratch slice at least `get_inplace_scratch_len()` long — a
  value **not** bounded by the transform length. For Bluestein-routed lengths (primes whose `len - 1`
  has a large prime factor like 83, or prime powers like 169/289) it *exceeds* the FFT length, so a
  pool sized only to `buffer_size` would make `process_with_scratch` panic. The **first** complex
  buffer is kept at exactly `buffer_size` (it's the in-place FFT *data* buffer — its length must be an
  exact multiple of the transform length, so it must not be widened); the remaining complex buffers
  are sized to `buffer_size.max(fft_scratch_len)`. The `buffer_size` field (the logical transform
  length) is what length assertions use; the real scratch never feeds an FFT, so it stays at
  `buffer_size`. Read the `new_for_fft` doc comment before changing buffer counts or sizing.
- `BufferPool` hands out disjoint slices via `split_at_mut`, so the number of buffers it owns (three
  complex, one real, set by `COMPLEX_BUFFER_COUNT` / `REAL_BUFFER_COUNT`) bounds how much scratch a
  hot path can hold at once. A stage needing more simultaneous scratch must raise those counts.
- Pooled buffers retain their previous contents when reused — callers must overwrite what they read,
  not assume zeroed scratch.
