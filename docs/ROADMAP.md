# Roadmap

This is the planned direction for `prism`, the shared DSP core of the q-lib audio workspace. The goal
is narrow: be the **minimal common base** the leaf crates (reed / warble / damper) and the maestro
engine stand on — the primitives and value types that would otherwise be duplicated — and nothing
more.

**Guiding principle: a primitive graduates here only when a 2nd consumer needs it.** prism grows by
*distillation*, not invention. Code is moved out of a leaf into prism when a second crate would
otherwise copy it; the origin leaf then re-exports it so its call sites don't churn. Domain traits
stay in the leaves. Keep this file current: check items off as they land (git is the detailed task
log).

---

## Current — distilled in ✅

The base layer as it exists today, distilled out of reed:

- **`Latency`** — the frames-based delay currency (`input` / `output` / `lookahead` frames +
  `total_frames` / `total_ms`). Every pipeline stage reports its delay in this type; maestro sums
  them and re-exports it as `AudioLatency`.
- **`Float`** — the `f32`/`f64` numeric abstraction (`Display + Debug + FloatCore + FftNum`) every
  DSP crate is generic over. reed re-exports it as `reed::float`.
- **`buffer`** — `BufferPool<T>` (allocation-free scratch, `Send`) + real/complex copy/convert
  helpers + `square_sum`. reed re-exports it as `reed::utils::buffer::*`.

Rationale for prism existing and for the primitive-vs-trait boundary is in ADR 0001.

---

## Next — distill targets

Primitives that still live in reed and are the likely next graduations, **each only when a second
consumer (warble / damper) actually needs it**:

- **Cached FFT plans** — the forward/inverse `Arc<dyn Fft<T>>` plan pair reed builds once and reuses.
  The first thing a second FFT-based leaf will want.
- **Windowing** — analysis window functions (Hann, etc.) shared by any framed/spectral stage.
- **Framing** — overlapping-window framing / hop machinery common to streaming DSP stages.

None of these are in prism yet. Adding one is a deliberate distill-in: move it out of reed, re-export
it from reed, generic over `Float`, allocation-conscious, and recorded as an ADR if the boundary is
durable.

## Possible — a shared `Processor` super-trait

Each leaf owns its domain trait today. **If** real duplication appears across the leaf interfaces, a
common `Processor` super-trait (the shared shape — e.g. reporting `Latency`, a `reset`) may graduate
into prism. This is explicitly *not* done speculatively: domain traits stay in the leaves until a
concrete cross-leaf need forces the abstraction. See ADR 0001.

---

## Cross-cutting principles (all of the above)

- Distill, don't invent — a primitive lands here only with a second consumer; the origin leaf
  re-exports it.
- Primitives + value types only; domain traits stay in the leaves.
- Stay generic over `Float`; keep the hot path allocation-free and `Send`.
- Stay standalone-buildable, with `rustfft` as the only runtime dependency.
