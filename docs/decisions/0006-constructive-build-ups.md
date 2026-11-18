# 0006. Constructive build-ups, owned by each element

Status: Accepted
Date: 2026-11-18

## Context

Current tools describe walls, slabs and roofs with shared composites picked
from a catalogue: a core, finishes and "other" skins. Composites multiply
(dozens per project), are hard to edit, and do not say how the element is
built: a timber frame wall shows no studs, a tiled roof no battens. To
renovate, users split elements into several stacked ones, and an opening
then has to cross several walls.

## Decision

- **Each wall, slab and roof owns its build-up.** It starts from a preset
  (rule 17) and is edited in place. There is no shared composite
  attribute.
- **Layers carry a constructive role**: load-bearing, insulation, air gap,
  membrane, lining, finish, covering.
- **A layer can hold framing** within its thickness: members of a material,
  width and spacing (studs in the insulation of a timber frame wall, joists
  in a floor). Framing is drawn in plan and section.
- **Systems imply layers and members**: a covering brings what it needs
  (tiles: battens and counter-battens; zinc: boarding), a traditional timber
  roof brings rafters, purlins and trusses. They are options of the
  element, switched on by the system chosen.
- **A slab is one element**; its floor finish can change from room to room.
- **Renovation is per layer**, in the same element.
- **Detail drawing is induced, not modelled**: at detail scales, the
  drawing shows what the build-up implies (overlapping tiles, fixings,
  laps) while the 3D model keeps one layer.

## Consequences

- One attribute fewer to manage, and no catalogue to clean up.
- Junction priority comes from the material of each layer, as before; the
  role adds meaning (an insulation layer stays continuous at a corner).
- Framing and induced layers need their own geometry and drawing rules, to
  be designed with the needs sheets of walls, slabs and roofs.
- Schedules can count framing members and induced layers without them
  being separate objects.
