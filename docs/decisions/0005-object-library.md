# 0005. Object library: parametric tools, presets and open content

Status: Accepted
Date: 2026-11-18

## Context

Studios need ready-to-use objects from day one: standard door blocks,
usual window frames, furniture, sanitary ware. No large open library of
parametric BIM objects exists. Objects downloaded as IFC are not parametric
in practice: their parameters live in the tool that made them, and the file
keeps a fixed shape with properties. Vendor libraries (GDL objects, Revit
families) cannot be reused.

## Decision

Three kinds of content, each with its own source:

1. **Architectural elements are parametric tools**, not library objects:
   walls, slabs, doors, windows, roofs, stairs, railings, curtain walls
   (TOOLS.md, rules 15 and 16). Their internal model uses the architect's
   vocabulary. IFC is an export target: at export, doors and windows are
   mapped to the standard IFC door and window type properties where they
   fit, and always carry explicit geometry.
2. **Presets** give each tool its standard sizes and sections (rule 17):
   data files, sorted by country, written and reviewed by architects from
   building practice and public standards. They live in this repository.
3. **Open content** covers furniture, equipment and planting: simple
   objects described as data (parameters, simple geometry, a drawn plan
   symbol), plus any IFC, glTF or OBJ file imported as a fixed object. The
   shared library lives in a separate repository under CC BY 4.0.
   - Objects may be authored with any tool, including Blender with Bonsai or
     FreeCAD; their output does not inherit the tool's license.
   - IfcOpenShell may be used in tooling to generate and check content,
     never shipped.
   - Every object records its source and license. Content under a
     no-derivatives license (CC BY-ND) and content from vendor or
     manufacturer libraries are not accepted. Manufacturer files a studio
     downloads stay in its own projects.

## Consequences

- No dependency on a vendor library, and no yearly library update.
- Presets are real work: each country needs its door, window and section
  tables, checked by architects. This is where contributors from each
  country matter most.
- The object description format is ours and must stay simple enough for an
  architect to write without programming.
- The IFC mapping of doors and windows only covers what IFC can express
  (a few predefined panel layouts); explicit geometry guarantees that every
  other tool still sees the right shape.
