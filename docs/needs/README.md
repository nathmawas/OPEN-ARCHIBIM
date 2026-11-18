# Needs map

One sheet per element family, written by architects in their own words. A
sheet says what the element must do before anyone designs how. It feeds the
test cases in [tests/cases](../../tests/cases/README.md) and the presets.

| Sheet | Status |
|---|---|
| [Walls](walls.md) | Draft |
| [Slabs](slabs.md) | Draft |
| [Roofs](roofs.md) | Draft |
| [Doors](doors.md) | Draft |
| [Windows](windows.md) | Draft |

## How a sheet is built

1. **The need**: what the architect draws, decides and delivers with it.
2. **Today's problems**: what goes wrong in current tools, from real
   projects. Describe the problem, never another product's interface.
3. **Parts and details**: what the element carries as options
   (TOOLS.md, rule 16).
4. **Presets**: the standard sizes it ships with, by country (rule 17).
   Values are added by architects with their source.
5. **Drawing**: how it shows in plan, section, elevation and 3D, by scale.
6. **IFC**: how it is exported.
7. **What we drop**: settings we refuse to carry.
8. **Open questions**: what an architect still has to decide.

A sheet is `Draft` until an architect maintainer has reviewed it, then
`Agreed`.
