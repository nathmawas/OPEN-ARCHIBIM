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
- Three or more walls meeting at one point: layers overlap, or walls run
  past the point, and the junction has to be redrawn by hand.
- Junction settings that are hidden: nothing shows why two walls joined, did
  not join, or were silently extended.
- The same window drawn differently in two walls because the insulation
  return is set on the window instead of on the wall.

## Parts and details

- **Build-up**, owned by the wall (TOOLS.md, rule 18): ordered layers, each
  with a constructive role (load-bearing, insulation, air gap, membrane,
  lining, finish), a material, a thickness and a renovation status
  (existing, to be demolished, new). Started from a preset such as
  "traditional masonry with plasterboard lining on insulation" or "timber
  frame", then edited in place.
- **Framing** within a layer: studs of a timber frame wall in its
  insulation, with width and spacing, drawn in plan.
- **Priority** of each layer, started from its role (load-bearing first,
  then insulation, then air gaps, membranes and finishes low) and
  adjustable per material. An unusual order is flagged with a warning,
  never refused: the architect decides.
- **Junctions** driven by layer priority; equal priorities meet on a mitre
  unless the user picks the wall in front, per junction. A junction is
  visible and selectable, with its state: joined, in conflict, or not
  joined. The user can forbid joining at one end of a wall (an expansion
  joint). A wall is never extended or shortened without showing it.
- **Nodes** of three or more walls are resolved like any junction, around
  the point, with no limit on the number of walls.
- **Top and bottom** linked to a story or set to a fixed level, with an
  offset. Layers share the wall's top and bottom by default; where a detail
  needs it, a layer can have its own (external insulation and render
  running down past the slab edge at the foot of a ground floor wall).
- **Details**, per face where it applies:
  - skirting;
  - cornice or moulding;
  - band course;
  - plinth course (a different finish at the foot of the wall);
  - coping on a low wall or parapet;
  - insulation return into the reveals of openings: the wall says which
    layers return, edge by edge (jambs, head, sill); the opening only says
    where its frame sits. The same window adapts to each wall it is placed
    in.

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
Each junction is exported as a path connection between the two walls, with
the end or point of each wall, the layer priorities and the cut line, so
another tool can rebuild the junction instead of guessing it.

## What we drop

- Separate objects for skirtings, cornices or copings.
- Manual junction cleanup tools.
- Junction results that depend on display settings (hatches, pens) or on
  the order of clicks.

## Open questions

- Which details need their own material and renovation status?
- Are curved and sloped walls in V1.0 or later?
