# Walls

Status: Draft

## The need

A wall is a build-up of materials, drawn along a line on a story. The
architect sets its build-up, its height relative to the stories, and how it
meets other walls. From it come the plan cut, the sections, the quantities
and the renovation drawings.

## Today's problems

- Junctions that have to be fixed by hand, layers that overlap or leave
  gaps at corners.
- Thin walls used to fake skirtings, cornices or copings: they clash with
  the real walls and break junctions.
- External insulation on an existing wall means deleting and redrawing the
  wall.
- Height settings that do not follow the stories when the project changes.

## Parts and details

- **Build-up**, owned by the wall (TOOLS.md, rule 18): ordered layers, each
  with a constructive role (load-bearing, insulation, air gap, membrane,
  lining, finish), a material, a thickness and a renovation status
  (existing, to be demolished, new). Started from a preset such as
  "traditional masonry with plasterboard lining on insulation" or "timber
  frame", then edited in place.
- **Framing** within a layer: studs of a timber frame wall in its
  insulation, with width and spacing, drawn in plan.
- **Junctions** driven by material priority; equal priorities meet on a
  mitre unless the user picks the wall in front, per junction.
- **Top and bottom** linked to a story or set to a fixed level, with an
  offset.
- **Details**, per face where it applies:
  - skirting;
  - cornice or moulding;
  - band course;
  - plinth course (a different finish at the foot of the wall);
  - coping on a low wall or parapet;
  - insulation return into the reveals of openings.

## Presets

- Common build-ups by country, as wall presets: masonry with internal or
  external insulation, concrete, timber frame, partition types, each with
  usual thicknesses.
- Usual skirting, cornice and coping profiles.

## Drawing

- Plan: each layer cut with its material hatch; contour and layer lines by
  scale; details shown when the scale allows.
- Section and elevation: layers, details, renovation graphics.
- Coarse scales show the wall as one outline with a single hatch.

## IFC

A wall with its build-up as a standard IFC layered wall, contained in its
story, with explicit geometry. Details are exported as parts of the wall.

## What we drop

- Separate objects for skirtings, cornices or copings.
- Manual junction cleanup tools.

## Open questions

- Which details need their own material and renovation status?
- Are curved and sloped walls in V1.0 or later?
