# 0003. GPU rendering, recompute only what changed

Status: Proposed (to be confirmed by measurements in the Stage 0 prototype)
Date: 2026-11-18

## Context

Heavy display is one of the main complaints about current BIM tools. Fluidity
must be designed in from the start: it cannot be added at the end.

## Decision

- Plan and 3D are both drawn on the GPU with `wgpu`.
- A command returns the elements it changed; only those and their junction
  neighbours are recomputed, on background threads.
- Shapes stay in GPU memory and are replaced only when they change.
- Repeated elements are drawn by instancing; detail depends on the scale;
  only what is on screen is drawn.
- A performance budget (see ARCHITECTURE.md) is checked by benchmarks in CI,
  on a reference project kept in the repository.

## Consequences

- Smooth navigation on large projects, and an interface that never freezes.
- More design work up front: change tracking and the junction graph must be
  right from the first wall.
- 2D drawing on the GPU (lines, hatches, text) is harder than with a classic
  2D library; the prototype checks that print quality is not compromised.
