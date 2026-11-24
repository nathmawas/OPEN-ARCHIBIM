# 0011. Material families sort the list, never constrain a layer

Status: Proposed
Date: 2026-11-24

## Context

The starting library grows past a dozen materials, and the architect
picks one for every layer of every wall, slab and roof. A flat list gets
long and mixes concrete, membranes and roof tiles. Libraries also tend to
multiply: a brick for each colour, a timber for each species, a render
for each binder, until nobody finds the one they want.

## Decision

- **Each material belongs to one family**: masonry (concrete included),
  insulation, timber frame and roof framing, metal frame, air, membrane,
  facade finish, cladding, interior lining, roof covering, other.
- **Families only sort.** The material lists show the families as
  headings, with their materials under them. Nothing is refused because
  of a family: any material goes on any layer.
- **The list opens on the family matching the layer's role**: masonry
  for a load-bearing layer, insulation for an insulation layer, air for an
  air gap, and so on. The other families follow in their usual order.
- **A material is one kind.** Its colour, bond, binder or species are
  parameters of that material, to come, never new materials.
- **Family and role are different things.** The role says what the layer
  does in the build-up (ADR 0006); the family says what the material is.
  A timber layer can carry loads, a brick layer can be a finish.
- **The starting library** holds generic materials in every family and a
  few traditional ones (stone, rammed earth, hemp and lime, straw, slate,
  thatch). Its names are language pack keys; a material the architect
  names shows as typed.

## Consequences

- Lists stay short to scan however large the library gets.
- The library stays small: one material per kind, waiting for material
  parameters instead of variants.
- The family is not yet written to IFC, and the library is the same in
  every country; both are to come.
