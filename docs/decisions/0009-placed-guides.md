# 0009. Guides placed by hand are project data, not building elements

Status: Proposed
Date: 2026-11-22

## Context

An architect sets a plan out against lines that are not part of the
building: a line parallel to a boundary at the setback distance, the axis
of a row of columns, a circle of a given radius around a tree. Drawn by
hand, these are the T-square, the ruler and the compass. The automatic
guide lines that come from an edge under the pointer vanish as the pointer
moves on, so they cannot hold a setting-out for the time of a project.

## Decision

- A guide is a line through two points (infinite) or a circle. It belongs
  to one story and is kept in the project, next to the building elements,
  in its own list (`Project::guides`).
- Guides are added, changed and removed by commands, like every change of
  the model, so they are undone and scripted the same way.
- Guides are drawing aids: never in the 3D view, never in quantities,
  never printed. The drawing tools snap to them as they snap to the
  automatic guide lines.
- The model refuses to remove a story that still has guides, as it does
  for any element. The interface has no story removal yet; when it gets
  one, it will remove the guides first, in the same undo step, so the
  architect is never blocked.

## Consequences

- A setting-out survives a session and an undo history.
- Every new reader of the project (IFC export, quantities, printing) must
  leave guides out of the building. The IFC file will keep them as
  non-physical annotations under their story, which other tools may
  ignore; this is not written yet.
- Guides of every story show in the one plan the interface has today; they
  will be filtered by story once views have their own story.
