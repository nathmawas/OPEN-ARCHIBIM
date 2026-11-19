# 0008. Elements keep the line or point they were placed by

Status: Proposed
Date: 2026-11-19

## Context

An architect sets a wall out from a line that matters on site: the outside
face of the concrete, the axis of a partition, the inside face of a lining.
A column is set out from its centre or a corner, and a beam from the middle
or an edge of its top face. When the build-up or the size changes later,
that line or point must stay where it was drawn. Until now a wall was
always stored by its left face, so a thicker wall grew on one side only.

## Decision

- A wall stores the points of its reference line and which line it is: the
  left face, the axis, the right face, or the left or right face of its
  load-bearing layers. The faces are computed from the line and the
  build-up (`Wall::left_face`).
- Walls join where their reference lines meet. A corner mitre runs through
  the points where the faces on each side meet, so walls of different
  thicknesses close without a gap.
- A column stores its placed point, which of the nine points of its section
  that is, and its turn. A beam stores its reference line, the point of its
  section that line runs through, and the level of that line.
- These are element parameters, edited in the palette like the others and
  undone like any other change.

## Consequences

- Changing a thickness, a build-up or a size never moves what the architect
  set out.
- The drawing tools place the clicks as they are; they no longer compute
  faces themselves.
- Every reader of a wall's geometry must go through its faces, not its two
  points. The geometry crate is the only one that does today.
- IFC export will write the reference line as the wall axis with an offset
  of the material layer set, which IFC supports directly.
