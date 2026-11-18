# Roofs

Status: Draft

## The need

A roof is a set of slopes with a build-up, edges and a covering system. The
architect sets its geometry from any reference plane, its edges one by one,
and the way it is built. From it come plans, sections, elevations and the
roof details.

## Today's problems

- The reference plane is always the bottom of the roof.
- Overhangs and layers cannot be set edge by edge.
- Non-planar slopes need another tool.
- Gutters, fascias and verges are separate objects added against the roof.
- The timber structure (rafters, purlins, trusses) is modelled by hand or
  not at all.
- Tiles, battens and boarding are drawn by hand in detail.

## Parts and details

- **Geometry**: slopes from any reference plane (top of covering, top of
  structure, underside); overhang set edge by edge; non-planar slopes in
  V1.4.
- **Build-up**, owned by the roof (TOOLS.md, rule 18): structure,
  insulation, membranes, lining, with renovation status per layer.
- **Structure system**: traditional timber (rafters, purlins, trusses),
  engineered trusses, concrete, steel. A timber system switches on its
  members, with sections and spacing.
- **Covering system**: tiles, slates, zinc, steel sheet, membrane. Each
  system brings what it implies: tiles bring battens and counter-battens,
  zinc brings boarding.
- **Edges** (TOOLS.md, rule 16), per edge: gutter, fascia, soffit, verge,
  ridge, hip, valley, eaves detail.
- **Openings in the roof**: skylights and dormers are their own tools,
  placed on the roof.

## Presets

- Common roof build-ups and structure systems by country.
- Covering systems with their usual battens, boarding and laps.

## Drawing

- Plan: outline, ridges, hips and valleys, structure on demand.
- Section: each layer, structure members cut, covering.
- Detail scales: what the covering implies (overlapping tiles, battens,
  fixings), drawn without being modelled in 3D.

## IFC

A roof with its slopes as standard IFC slabs, its build-up as layer sets,
structure members as standard IFC members, explicit geometry.

## What we drop

- Separate objects for gutters, fascias and verges.
- A separate shell tool for warped slopes.

## Open questions

- Is the structure system part of the roof, or its own element linked to
  it?
