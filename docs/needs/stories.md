# Stories and levels

Status: Draft

## The need

The building is organised by stories. Every element is placed on a story
and its heights refer to story levels. When a level changes, everything
linked to it follows.

## Today's problems

- One single level per story: the difference between the finished floor and
  the top of the structure is handled by hand, element by element.
- Walls, columns and stairs that stop at the wrong level once floor
  build-ups are known, because they were linked to the finished floor.

## Parts and details

- **Finished level**: top of the finished floor. It is the story's level,
  the one shown in sections and used for sill heights.
- **Structural level**: top of the structural slab, below the floor
  build-up (screed, insulation, finish). By default it is the finished
  level minus the story's floor build-up; it can be set directly.
- Every height can be linked to either level of any story, with an offset:
  a wall stands on the structural level and rises to the structural level
  of the story above minus the slab; a sill is measured from the finished
  level.
- The floor build-up can differ from one room to another; the story keeps
  a default.

## Presets

- Usual floor build-up depths by country and floor type.

## Drawing

- Sections and elevations show both levels when the scale allows, with the
  usual level marks.
- Plans state the finished level of the story.

## IFC

The story's elevation is its finished level. The structural level is
exported as a story property.

## What we drop

- Several stories created only to handle a finished and a structural level.

## Open questions

- Is the structural level shown in plans as well?
