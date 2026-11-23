# 0010. A cut is a relation kept in the project, not a new shape

Status: Proposed
Date: 2026-11-23

## Context

A wall that rises past a roof has to stop on its underside, following its
slopes, its ridge and its gables. Drawing that top by hand, or giving the
wall a new height, breaks as soon as the roof changes: the slope, the
overhang or the eaves level. The same need will come for walls under a
slab, and for the operations between any two volumes (one taken out of
another, or only what two volumes share).

## Decision

- A cut is a relation between two elements kept in the project: the
  element cut (a wall, a column or a beam) and the element that cuts it (a
  roof, for now). It lives in its own list in the project, next to the
  junctions between walls.
- A cut changes no parameter of the element cut. The wall keeps its own
  height; its 3D shape is worked out from the wall and the roof each time,
  so the cut follows the roof when it changes.
- Cuts are set and released by a command, undone like any change. Removing
  either element removes its cuts; undoing the removal brings them back in
  the same step.
- Cutting under a roof keeps what is under the roof's underside, pane by
  pane, and leaves the rest of the element as it was. It never stretches
  an element up to the roof. The plan does not change.

## Consequences

- The cut is never lost or stale when the roof is edited, moved or
  deleted; deleting the roof gives the walls back their whole height.
- Every reader of the shape (3D, sections, quantities, IFC export) must
  apply the cuts. In IFC a cut becomes a clipped body: the element's solid
  less one bounded half-space per roof pane. The relation itself has no
  place in IFC, so another program reading the file gets the shape only.
- The 3D view works the cuts out again on every change, like everything
  else; once recompute becomes incremental (ADR 0003), changing a roof
  must also redo the elements cut under it, which the command already
  reports.
- Other cuts (under the roof's top or layer by layer, under a slab, the
  general operations between volumes) extend this relation with a mode,
  rather than adding a new mechanism.
