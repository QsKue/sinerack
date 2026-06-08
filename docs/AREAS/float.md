# Float

Source: `src/float.rs`

A single trait, `Float`, that acts as a generic stand-in for `f32` or `f64` so every DSP crate in the
workspace can be written once and used at either precision. It was distilled out of reed, which now
re-exports it as `reed::float`.

```rust
pub trait Float: Display + Debug + NumFloatCore + FftNum {}
impl Float for f64 {}
impl Float for f32 {}
```

## What belongs here

- The `Float` trait definition and its blanket impls for the supported primitive float types.
- Trait bounds that are genuinely needed by *every* consumer of `Float`.

## Patterns to follow

- Public APIs across the workspace are generic over `T: Float`. When a function needs more than the
  base bounds (e.g. `std::iter::Sum` for `square_sum`), add that bound *at the use site*, not to the
  `Float` trait itself — keep `Float` minimal.
- The bounds come from what the code actually does: `NumFloatCore` (arithmetic, `infinity`,
  `is_sign_negative`, `from_usize`/`from_f64`), `FftNum` (required by `rustfft`), and
  `Display + Debug` (used in tests/diagnostics).

## What should not be placed here

- Algorithm logic or buffer code. This module is purely the precision abstraction.

## Gotchas

- `FftNum` is the binding constraint that limits which types can be `Float` — a candidate type must
  be usable as an FFT scalar in `rustfft`. That's why only `f32`/`f64` are implemented.
