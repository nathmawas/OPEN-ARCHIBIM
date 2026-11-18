# Slabs

Status: Draft

## The need

A slab is a build-up of layers drawn as an outline on a story: floors,
terraces, landings, ground slabs. The architect sets its build-up, its
level and its edges. From it come floor areas, sections and the edge
details of facades and terraces.

## Today's problems

- Floor build-ups split between several slabs (structure, insulation,
  screed, finish) that drift apart when the outline changes.
- Slab edges with no detail: nosings, drips and edge insulation are drawn
  by hand in section.
- Openings in slabs (stairwells, shafts) that lose their relation to what
  passes through them.

## Parts and details

- **Build-up**, owned by the slab (TOOLS.md, rule 18): ordered layers with
  role, material, thickness and renovation status, as for walls; joists as
  framing for timber floors.
- **Floor finish**: an option of the slab. The slab is one element; its
  finish can change from one room to another.
- **Level** linked to a story's finished level or structural level, with
  an offset (see [Stories and levels](stories.md)).
- **Voids**: stairwells and shafts, with their trimming.
- **Edge details**, per edge:
  - edge trim or facing;
  - drip;
  - thermal break;
  - waterproofing upstand on a terrace;
  - threshold where the slab meets a door.

## Presets

- Common floor build-ups by country: concrete slab, timber floor, ground
  slab, terrace.
- Usual edge trims and upstand heights.

## Drawing

- Plan: shown below the cut plane with the finish; edges and openings.
- Section: each layer, edge details, renovation graphics.

## IFC

A slab with its build-up as a standard IFC layered slab, contained in its
story, with explicit geometry. Openings are standard IFC openings.

## What we drop

- Several stacked slabs to build one floor.

## Open questions

- Are sloped slabs in V1.0 or with roofs in V1.1?
