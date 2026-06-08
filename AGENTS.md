# AGENTS.md

`prism` is the small, shared **DSP core** of the q-lib audio workspace (edition 2024,
`AGPL-3.0-or-later`). It is the base layer everything else stands on: it holds the DSP **primitives**
and common **value types** that would otherwise be duplicated across the leaf crates. No binary, no
async, no runtime, no I/O. Today it carries three things: `Latency` (the frames-based delay currency
every stage reports in), `Float` (the `f32`/`f64` numeric abstraction), and `buffer` (a pre-allocated
scratch `BufferPool` + real/complex copy/convert helpers). Its only runtime dependency is `rustfft`.
Keep changes minimal, generic, and allocation-conscious.

**Consumer / workspace context.** prism sits at the bottom of the three-layer q-lib audio system: it
has no dependencies of its own among the audio crates — it is depended *on*. The leaf DSP crates —
[`reed`](https://github.com/QsKue/reed) (pitch detection), `warble` (time-stretching), and `damper`
(denoising) — each own their domain trait and depend on prism for primitives. The
[`maestro`](https://github.com/QsKue/maestro) engine orchestrates the leaves and is the **hub / main
entry point** for the whole audio system; it re-exports `prism::Latency as AudioLatency`. prism's
`Float` and `buffer` were just **distilled out of reed**, which now re-exports them
(`reed::float` = `prism::Float`, `reed::utils::buffer::*` = `prism::buffer::*`). All crates are git
submodules + path workspace members. prism must stay standalone-buildable and must not gain leaf or
engine knowledge.

## Where to look

- `docs/ARCHITECTURE.md` — module tree (latency / float / buffer), what each provides, prism's role
  as the shared base, the check commands, and what is distilled-in-now vs planned. Read before
  touching module boundaries or the public API.
- `docs/AREAS/*.md` — per-module conventions and gotchas. Read the one for any file you change.
- `docs/DECISIONS/*.md` — durable design decisions with rationale (ADRs).
- `docs/ROADMAP.md` — the phased distill plan and what graduates here next.

## Architecture in one screen

- `src/lib.rs` — crate root; declares modules and re-exports `Latency` + `Float`; `buffer` is a
  public module.
- `src/latency.rs` — the `Latency` value type: a three-field (`input_frames` / `output_frames` /
  `lookahead_frames`) frames-based delay struct + `total_frames` / `total_ms`. The shared currency
  every pipeline stage reports in.
- `src/float.rs` — the `Float` trait (`Display + Debug + FloatCore + FftNum`) abstracting `f32`/`f64`,
  with blanket impls. The single numeric bound every q-lib DSP crate is generic over.
- `src/buffer.rs` — `BufferPool<T>` (pre-allocated real/complex scratch, lent out as disjoint mutable
  slices) + real/complex copy/convert helpers (`copy_real_to_complex`, `modulus_squared`, …) +
  `square_sum`. The allocation-free scratch management FFT-based stages share.

## Conventions (the durable rules)

- **Primitives + value types only.** prism holds shared DSP machinery (`buffer`) and shared value
  types (`Latency`, `Float`) — nothing domain-specific. **Domain traits stay in the leaves**
  (`PitchDetector` in reed, `TimeStretcher` in warble, `Denoiser` in damper), never in prism, so an
  unrelated leaf crate is never dragged into a lockstep version bump. A common `Processor` super-trait
  may graduate here later *only if* real cross-leaf duplication appears — not speculatively.
- **A primitive graduates here only when a 2nd consumer needs it.** Code is distilled *out of* a leaf
  into prism when a second crate would otherwise copy it (this is how `Float`/`buffer` arrived from
  reed). Don't add speculative primitives. Distilled code is re-exported from the origin leaf so its
  call sites don't churn.
- **Stay generic over `Float`.** Public APIs take `T: Float`, never concrete `f32`/`f64`. Add extra
  bounds (e.g. `std::iter::Sum` on `square_sum`) only at the use site that needs them, never on the
  `Float` trait itself — keep `Float` minimal.
- **Allocation discipline (`BufferPool`).** The pool allocates every buffer once in `new` /
  `new_for_fft` and lends disjoint mutable slices (`complex_pair` / `complex_triple` / `real`) on the
  hot path — never `vec!` in a borrow. It owns its buffers outright (no `Rc`/`RefCell`), so it stays
  `Send`. This allocation-free, `Send` contract is the reason `buffer` lives here; preserve it.
- **Stay small + standalone.** No I/O, threading, async, audio-device, or engine/leaf concepts. The
  runtime dep surface is FFT only (`rustfft`, `default-features = false`). prism must build on its own.

## Warning signs

- A type or trait in prism mentions pitch, time-stretch, denoise, or any single leaf's domain.
- A domain trait (`PitchDetector`, `TimeStretcher`, `Denoiser`) is moved into prism "to share it" —
  it belongs in its leaf unless a real `Processor` super-trait is being deliberately graduated.
- A `BufferPool` method allocates in a borrow instead of handing out a pre-allocated slice.
- A primitive is added to prism that only one crate uses (premature distillation).
- A change makes `f32` work but silently breaks `f64` (or vice versa) — both must stay supported.

## Making a change

1. Read `docs/ARCHITECTURE.md` (if touching boundaries / the public API) and the relevant
   `docs/AREAS/*.md`.
2. Keep the change small, generic, and allocation-conscious; keep doc updates near the behavior change.
3. Run the checks in `docs/ARCHITECTURE.md` (fmt / clippy / test / doc). When distilling a new
   primitive in, update the origin leaf's re-export and add a `docs/DECISIONS/` ADR if it's durable.

## Docs maintenance

- **Code is truth for behavior; docs explain why and what-not-to-do, not line-by-line how.**
- **Git is the task log** — there is no task-history / changelog directory, and you should not create
  one (it only duplicates `git log`).
- Update the smallest useful set: `docs/AREAS/*` for a changed convention/gotcha (one file per real
  module — keep `Source:` paths honest), `docs/ARCHITECTURE.md` for module boundaries / API shape, a
  new `docs/DECISIONS/` ADR (from `docs/TEMPLATES/decision-template.md`) for a durable choice,
  `docs/ROADMAP.md` for plan and status. Keep every doc short enough to read at task start.
