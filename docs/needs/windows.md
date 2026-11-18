# Windows

Status: Draft

## The need

A window is placed in a wall and composed of parts: frame, sashes, fixed
lights, transom, and what goes around it (sill, shutters, lintel). The
architect sets its composition, its position in the wall depth, its sill
height, and describes it in schedules and elevations.

## Today's problems

- Combined windows built from several objects placed side by side.
- Sills, shutters and roller shutter boxes added as separate objects that
  do not follow the window.
- The position of the frame in the wall depth and the insulation return in
  the reveals drawn by hand in section.

## Parts and details

- **Frame**: profile, position in the wall depth (in the insulation, at the
  inner face, at the outer face).
- **Composition** (TOOLS.md, rule 15): any arrangement of sashes and fixed
  lights, transom, side lights; each part has its own operation
  (side-hung, tilt-and-turn, sliding, pivot, top-hung, fixed).
- **Sizes**: nominal size chosen by the user, as for doors.
- **Around the window**:
  - outer sill (stone, concrete, metal) with overhang and drip;
  - inner board;
  - lintel;
  - roller shutter, with its box (inside, outside or in the lintel) and
    its guides;
  - hinged shutters;
  - guard rail or glazed lower panel;
  - air inlet;
  - insulation return in the reveals.

## Presets

- Usual frame profiles and sections by material (timber, PVC, aluminium,
  steel).
- Common window sizes by country.
- Usual sills and shutter boxes.

## Drawing

- Plan: frame, glazing and sill by scale, opening symbol on demand.
- Elevation: division, opening symbols, shutters.
- Section: frame position, sill, lintel, insulation return.

## IFC

A window with its type, the standard partitioning when it fits, frame and
panel properties where IFC can express them, and explicit geometry. Shutters
and sills are exported as parts of the window.

## What we drop

- Window object catalogues and side-by-side assemblies.
- Separate objects for sills and shutters.

## Open questions

- Is joinery one tool with door and window presets, or two tools sharing
  one model? (see [Doors](doors.md))

Settled:

- Skylights are a tool of their own, placed in a roof: it is roofer's work,
  not joinery. Dormers get their own parametric tool too (TOOLS.md).
- A glazed door is both a door and a window: doors and windows are one
  family, exterior or interior joinery, sharing one composition model.
