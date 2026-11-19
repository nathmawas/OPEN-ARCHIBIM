# 0007. egui for the interface, drawn through wgpu

Status: Proposed (to be confirmed by the Stage 0 prototype)
Date: 2026-11-19

## Context

ADR 0001 left the interface toolkit to the prototype, between `egui` and
`iced`. The prototype must show a plan quickly, keep the interface fluid,
and draw through `wgpu` (ADR 0003).

## Decision

- `egui` for windows, palettes and the plan view; `eframe` opens the window
  and draws everything through its `wgpu` back end.
- `egui` stays inside `archibim-ui`; `eframe` stays inside `archibim-app`.
  The model, geometry and command crates never see them.
- The plan is drawn with egui shapes for now. When the performance budget
  needs it, the plan and 3D views move to our own `wgpu` pipeline inside an
  egui panel, as ADR 0003 describes.
- The 3D view uses that pipeline from the start, in `archibim-render`: it
  needs a depth buffer, which egui shapes do not have.

## Consequences

- A visible prototype in days: egui redraws the whole interface each frame,
  with no state to keep in sync with the model.
- egui's look is plain. Our own icons (ADR 0004) and a theme come later.
- Redrawing every frame costs power on large projects; the plan view must
  cache its shapes before that shows.
- About 170 crates come with eframe, all under permissive licenses (see
  PROVENANCE.md).
