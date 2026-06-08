//! Generic [`Float`] type which acts as a stand-in for `f32` or `f64`.
use rustfft::FftNum;
use rustfft::num_traits::float::FloatCore as NumFloatCore;
use std::fmt::{Debug, Display};

/// Signals are processed as arrays of [`Float`]s. A [`Float`] is normally `f32`
/// or `f64`. This is the shared numeric abstraction every q-lib DSP crate is
/// generic over, so detectors, stretchers, and denoisers all work with either
/// precision without re-declaring the bound.
pub trait Float: Display + Debug + NumFloatCore + FftNum {}

impl Float for f64 {}
impl Float for f32 {}
