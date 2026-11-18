# Doors

Status: Draft

## The need

A door is placed in a wall and composed of parts: frame, leaves, side
panels, fanlight. The architect chooses a standard door block, checks its
clear width against accessibility rules, sets its opening direction, and
describes it in schedules.

## Today's problems

- A catalogue of door objects: adding a side panel or a fanlight means
  starting again from another object and losing the settings.
- Several sizes (wall opening, frame, clear width, leaf) with no clear
  statement of which one is the nominal size.
- Accessibility checks done by hand: clear width and manoeuvring space are
  not shown.

## Parts and details

- **Door block**: frame, one or two leaves, threshold, casings per face.
- **Composition** (TOOLS.md, rule 15): side panel or semi-fixed leaf,
  fanlight, vision panel; each part has its own operation.
- **Operation**: side-hung push or pull, double, sliding (surface or
  pocket), folding.
- **Sizes**: the user chooses the nominal size (wall opening, frame
  outside, clear opening, leaf); the others are calculated and shown.
- **Options**: undercut or ventilation grille, door closer, handle side.
- **Accessibility**: clear width, and the manoeuvring space as an option of
  the door, shown in plan on demand.
- **Doors and windows are one family.** An architect thinks "exterior
  joinery" (or interior joinery), not "door" or "window": a glazed door is
  both. Doors and windows share the same composition model; door options
  (threshold, accessibility) apply to any part that reaches the floor.

## Presets

- Standard door blocks by country, with their clear widths: the usual
  leaf series, double doors, doors with a side leaf.
- Accessibility widths by country and building type.
- Usual frame and leaf sections.

Values are added by architects with their source (standard, regulation or
manufacturer practice).

## Drawing

- Plan: leaf and swing by scale, threshold, casings at large scales,
  manoeuvring space on demand.
- Elevation: leaf division, vision panel, opening symbol.
- Coarse scales show a simplified symbol.

## IFC

A door with its type, the standard door operation when it fits, frame and
panel properties where IFC can express them, and explicit geometry.

## What we drop

- Door object catalogues.
- Separate settings for each graphic detail of the symbol: one view style
  decides.

## Open questions

- Is joinery one tool with door and window presets, or two tools sharing
  one model?

Settled: the manoeuvring space is a door option. Presets start with France,
kept as close as possible to widely used models; other countries come from
their communities.
