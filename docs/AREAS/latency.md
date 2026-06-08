# Latency

Source: `src/latency.rs`

The `Latency` value type — the shared, frames-based delay currency every pipeline stage in the audio
system reports in, so maestro can sum the latency of analyzers, denoisers, time-stretchers, and
processors into one end-to-end figure. maestro re-exports it as `AudioLatency`.

```rust
pub struct Latency {
    pub input_frames: usize,      // input frames required before useful output can begin
    pub output_frames: usize,     // frames retained / delayed on the output side
    pub lookahead_frames: usize,  // extra analysis/control lookahead before a decision is available
}
```

## What belongs here

- The `Latency` struct and its constructors/accessors: `const fn new`, `const fn total_frames` (the
  sum of all three components), and `total_ms(sample_rate)`.
- Future *frames-based, domain-agnostic* delay accounting, if any. Each field is a distinct **kind**
  of delay so stages can be reasoned about and summed independently — keep new fields in that spirit.

## Patterns to follow

- Latency is measured in **frames**, not milliseconds or seconds — frames are sample-rate-independent
  and additive across stages. Convert to time only at the edge with `total_ms`.
- It is a plain `Copy` value type (`Clone, Copy, Debug, Default, PartialEq, Eq`). Keep it cheap and
  trait-light; no allocation, no FFT, no `Float`.

## What should not be placed here

- Anything stage-specific (a particular detector's or stretcher's delay math) — those compute a
  `Latency` and return it; this type just *carries* the figure.

## Gotchas

- `total_ms` returns `None` when `sample_rate == 0` (avoids a divide-by-zero), rather than panicking
  or returning `0.0`. Callers must handle the `None`.
- `total_frames` is the simple sum of the three fields; the split exists for reasoning, not for any
  weighting — don't assume one field dominates.
