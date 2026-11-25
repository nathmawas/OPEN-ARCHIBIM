# 0012. A black and white interface, Arial, icons read by the interface

Status: Proposed
Date: 2026-11-25
Supersedes: the part of ADR 0004 on generic actions

## Context

Architects spend their day on drawings: black lines on white paper. The
interface used the toolkit's default theme (ADR 0007 left the theme for
later) and words in place of icons. The maintainer drew the first icons
by hand and asked for an interface in black and white, with colour only
where it helps, and Arial to begin with.

How the model itself is shown, in plan and in 3D, is a separate question:
it is not decided here.

## Decision

- **Black on white.** Panels, menus and buttons are white with black
  text and lines, square corners and no shadows. What is on (the tool
  in use, an aid switched on) turns black with white text, like an
  inked box.
- **Arial**, read from the system, since the font is not ours to ship;
  Liberation Sans, drawn to the same widths, stands in where Arial is
  missing, and the default font after it.
- **Icons read by the interface.** The SVG files of `assets/icons` are
  read at run time by a small reader of our own, limited to the subset
  of the style guide (`assets/icons/README.md`), and drawn as strokes in
  the colour of the button. No SVG library is added.
- **Generic actions drawn by us too.** Undo, save, zoom and the like are
  drawn in the same style rather than taken from an open icon set, so
  every icon reads as one family. This replaces that point of ADR 0004.

## Consequences

- An interface close to the architect's own drawings.
- An icon outside the subset is refused by a test, not drawn wrong.
- The plan and the 3D view keep their colours until the display of the
  model (pens, line weights, fills) is studied and decided.
- The look and the font are fixed for now; choosing them comes with the
  interface preferences.
