# Architecture

This document describes the structure of the `prism` crate. Keep it aligned with `AGENTS.md` and
update it when the public API, module boundaries, or what's distilled-in change.

## Shape

`prism` is the base layer of the q-lib audio workspace — the shared DSP core. It is a single, small
library crate with a flat module tree:

```text
lib.rs        crate root: module declarations + `Latency` / `Float` re-export
├── latency   the `Latency` value type (frames-based delay currency)
├── float     the `Float` trait (f32 / f64 abstraction)
└── buffer    BufferPool + real/complex copy/convert + square_sum
```

prism holds **primitives** (`buffer`) and **shared value types** (`Latency`, `Float`) — the things
that would otherwise be copied across the leaves. It has no engine, no async, no I/O, and no
domain-specific concepts. Its only runtime dependency is `rustfft`.

## Role as the shared base

The q-lib audio system is three layers, with prism at the bottom:

- **prism (this crate)** — primitives + value types. Depends only on `rustfft`.
- **reed / warble / damper (leaves)** — each owns its own domain trait (`PitchDetector` /
  `TimeStretcher` / `Denoiser`) and depends on prism. Domain traits live in the leaves, **not** in
  prism, so unrelated leaves don't get dragged into lockstep version bumps.
- **maestro (engine)** — orchestrates the leaves; re-exports `prism::Latency as AudioLatency`. It is
  the hub / main entry point for the audio system. See <https://github.com/QsKue/maestro>.

prism's `Float` and `buffer` were distilled out of reed; reed re-exports them (`reed::float` =
`prism::Float`, `reed::utils::buffer::*` = `prism::buffer::*`), so existing call sites are unchanged.

## Public API

The public contract is intentionally tiny:

- `Latency` (re-exported at the crate root) — `{ input_frames, output_frames, lookahead_frames }`
  with a `const fn new`, `total_frames()`, and `total_ms(sample_rate)`. Each pipeline stage reports
  its delay as a `Latency`; the engine sums them into one end-to-end figure.
- `Float` (re-exported at the crate root) — `Display + Debug + FloatCore + FftNum`, the trait callers
  parameterize on (`T: Float`) instead of picking `f32`/`f64`.
- `buffer::*` (public module) — `BufferPool<T>` (`new` / `new_for_fft`, `complex_pair` /
  `complex_triple` / `real`) plus the free helpers `new_real_buffer`, `new_complex_buffer`,
  `copy_real_to_complex`, `copy_complex_to_real`, `modulus_squared`, and `square_sum`.

## Key design properties

- **Generic over precision.** Everything is parameterized by `T: Float`. Extra bounds (e.g.
  `square_sum`'s `std::iter::Sum`) are added at the use site, not on the `Float` trait. Do not
  collapse to a concrete float type.
- **Allocation reuse.** `BufferPool` owns pre-allocated real/complex scratch buffers, lent out as
  disjoint mutable slices, so a stage's hot path borrows scratch instead of allocating per call. This
  is a load-bearing property; preserve it.
- **`Send`.** The pool owns its buffers outright (no `Rc`/`RefCell`), so it — and anything holding one
  — is `Send`, movable into another thread or a real-time audio callback.

## Distilled-in now vs planned

- **In prism today:** `Latency`, `Float`, `buffer` (`BufferPool` + copy/convert helpers +
  `square_sum`).
- **Not yet (future distill targets):** cached FFT plans, windowing, and framing. These still live in
  reed; they graduate here only when a second consumer needs them. See `docs/ROADMAP.md`.

## Testing & checks

prism has no audio hardware and a tiny surface. `buffer` carries inline unit tests (`BufferPool`
slice handout); the doc comment on `BufferPool` is a runnable doctest.

```bash
cargo test                                 # unit + doc tests
cargo fmt --all --check                    # formatting
cargo clippy --all-targets -- -D warnings  # lints (CI gates on this)
cargo doc --no-deps                        # docs
cargo +1.86.0 check --all-targets          # MSRV (edition 2024 needs >= 1.85)
```

CI (`.github/workflows/main.yaml`) runs the fmt / clippy `-D warnings` / build / test gate on stable
plus an MSRV check.

## Documentation coupling

When you change a module's responsibility, update the matching `docs/AREAS/*` file; when you change
the public API or what's distilled-in, update this document and add a `docs/DECISIONS/` ADR if the
choice is durable. See `AGENTS.md` for the full docs-maintenance policy.
