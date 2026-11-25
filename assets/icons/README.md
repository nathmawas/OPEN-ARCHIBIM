# Icon style guide

Our own icons, drawn by the project (ADR 0004). This guide keeps every
icon in one hand, whoever draws it.

## The look

Black line on white, like a drawing on tracing paper: sober, thin, exact.
An icon is a small architectural drawing, not a picture.

- **One stroke weight.** 1.5 on the grid, round ends and round corners.
- **No fill, no shadow, no gradient.** The colour is `currentColor`; the
  interface sets it (black, or white on a selected button).
- **A 24 x 24 grid**, drawn inside the safe zone from 2 to 22. Keep at
  least 3 units of air on the side a shape does not need.
- **Few shapes.** One idea per icon, five shapes at most. It must read at
  20 pixels.
- **The view an architect would draw.** Elevation for what stands (wall,
  window, door, column), section for what spans (a beam is its profile),
  profile for a stair, plan for a slab, a tilted sheet for terrain.

## The grammar

The same mark always means the same thing.

| Mark | Meaning | How |
| --- | --- | --- |
| Solid line | The element itself, or the result | default |
| Dashed line | What is virtual: a guide, a ghost, the position before or after | `stroke-dasharray="2 2"` |
| Dash-dot line | An axis, or an area taken whole | `stroke-dasharray="4 2 1 2"` |
| Small circle | A point: an end, a node, a grip | `r="1.5"` |
| Two short ticks | A division of a line | lines of length 3 across it |
| Open arrowhead | A movement or a direction | two lines of length 3 |
| Right-angle mark | Perpendicular | a 2.5 square corner |

## The file format

The interface reads the icons itself, so a file uses only this subset:

- the root `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"
  fill="none" stroke="currentColor" stroke-width="1.5"
  stroke-linecap="round" stroke-linejoin="round">`;
- inside it, only `<line>`, `<rect>`, `<circle>`, `<polyline>`,
  `<polygon>` and `<path>`;
- a `<path>` uses absolute commands only: `M`, `L`, `C`, `Q`, `Z`;
- the one attribute allowed beyond the geometry is `stroke-dasharray`;
- numbers are plain decimals; no `transform`, no `style`, no groups.

The file name is the tool or the aid in English, in kebab case:
`wall.svg`, `snap-midpoints.svg`, `edit-rotate.svg`.

## Before adding an icon

- It is drawn for this project, from the need, never traced or copied
  from another product.
- It follows the grammar above; a new mark is added to the table first.
- It reads at 20 pixels next to its neighbours.
