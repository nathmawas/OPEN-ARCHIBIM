# OPEN-ARCHIBIM Roadmap

Every stage ends with **something an architect can try**. The scope of each
stage comes from the tool review in [TOOLS.md](TOOLS.md), sections 3 and 4.

Guiding principles: see [MANIFESTO.md](MANIFESTO.md).

---

## At a glance

| Stage | What you can do at the end | Tools | Duration | Indicative window |
|---|---|---|---|---|
| **0. Foundations** | Watch a prototype video, read the project, contribute | — | 2 months | Dec 2026 → Jan 2027 |
| **V1.0 The core** | Draw a simple house in plan and 3D, exchange it as IFC | 39 | 4 months | Feb → May 2027 |
| **V1.1 The complete building** | Roof, stairs, railings, furniture, terrain, rooms, renovation | 14 | 4 months | Jun → Sep 2027 |
| **V1.2 The drawing** | Plans, sections and elevations legible without retouching | 30 | 3 months | Oct → Dec 2027 |
| **V1.3 The permit set** | Submit a real building permit application, published as PDF and DWG | 21 | 3 months | Jan → Mar 2028 |
| **V1.4 Free forms** | Warped roofs, sculpted volumes, curtain walls | 5 | 3 months | Apr → Jun 2028 |
| **V2** | Coordination with project partners, rendering, details, revisions | 25 | 9 to 12 months | mid-2028 → 2029 |
| **V3** | Several people on one project, MEP, structure, energy | 13 | to be defined | from 2029 |

**Key milestone: March 2028.** This is the first version a studio can use to
produce a building permit application from start to finish (V1.0 to V1.3,
about 15 months after the start).

```mermaid
gantt
    dateFormat YYYY-MM
    axisFormat %b %Y
    section Foundations
    Stage 0                 :p0, 2026-12, 2M
    section V1
    V1.0 The core           :v10, after p0, 4M
    V1.1 Complete building  :v11, after v10, 4M
    V1.2 The drawing        :v12, after v11, 3M
    V1.3 The permit set     :crit, v13, after v12, 3M
    V1.4 Free forms         :v14, after v13, 3M
    section Next
    V2                      :v2, after v14, 10M
```

---

## The team behind these durations

The durations assume a **core team**:

- **2 full-time developers**, assisted by AI;
- **1 half-time architect**, who settles design questions and tests each
  stage.

They also assume we write our own building model and reuse open libraries
for the rest (see "Technical approach" below). They change a lot with
resources:

| Situation | Effect on V1 (V1.0 → V1.4) |
|---|---|
| Core team | ≈ 19 months, as above |
| A single developer | × 2 to × 3: 3 to 4 years |
| 4 developers or more | ≈ 12 months at best: coordination limits the gain |

These are orders of magnitude. They will be revised at the end of each stage
by comparing actual time with planned time.

---

## Stage 0 — Foundations *(Dec 2026 → Jan 2027)*

Goal: a readable, credible project, and a settled technical choice.

- [x] MIT license
- [x] Manifesto
- [x] Tool review: [TOOLS.md](TOOLS.md) (findings, design rules, review of
      162 tools, V1 stages)
- [ ] **Two-week prototype** of our own core (see "Technical approach"):
      stories, a composite wall with junction priorities, one composable
      window; then the choice of stack and libraries
- [ ] **Needs map**: for each tool family, a sheet written **in our own
      words** (the need, today's problem, what we drop), fed by real studio
      cases
- [ ] `PROVENANCE.md`: where each engine comes from (public specification,
      reused code and its license)
- [x] [CONTRIBUTING.md](CONTRIBUTING.md): clean-room rule, "concrete gap" issue template,
      review by an architect of every design feature, translation guide
- [ ] 10 architects interviewed about their three biggest frustrations
- [ ] Prototype video, launch of the funding page

**Deliverable:** a documented public repository, the chosen stack, and
a first call for contributors and funding.

## V1.0 — The core *(4 months)*

A simple house (without a roof) drawn in plan and 3D, saved and read back as
IFC. This is also where the foundations of everything else are laid:

- stories, grids, composite walls with **junctions driven by material
  priority**, slabs, doors, windows, openings, columns, beams;
- **linked types and instances**, one single material definition;
- **synchronised windows** for plan and 3D;
- **command line** and **MCP server**: every action is a command;
- **native IFC**, saving;
- **translation from day one**: no text is hard-coded. English and French
  ship with V1.0.

**Deliverable:** the house, read back without errors by other IFC tools.

## V1.1 — The complete building *(4 months)*

A redesigned roof (any reference plane, overhang set edge by edge,
independent layers), stairs (helical with no angle limit), railings, objects
and furniture, terrain, rooms and areas, and **layer-based renovation**
(external insulation added onto the existing wall).

**Deliverable:** first trials in 2 or 3 pilot studios, on one real new-build
project and one renovation.

## V1.2 — The drawing *(3 months)*

One view style, named line weights, fonts by purpose, hatches, dimensions,
texts and labels, sections and elevations, reference overlay, multi-selection
eyedropper.

**Deliverable:** plans, sections and elevations of a house, ready to print
without retouching.

## V1.3 — The permit set *(3 months)*

Layouts, title blocks, PDF and DWG publishing, schedules, area tables,
properties and classifications, other partners' models as references.
Spanish, Italian, German and Portuguese language packs complete the first
six languages.

**Deliverable: the first usable version.** A real building permit application
submitted by a pilot studio, produced entirely in OPEN-ARCHIBIM.

## V1.4 — Free forms *(3 months)*

Non-planar roofs, free-form volumes and Boolean operations, curtain walls,
drawn profiles.

## V2 *(9 to 12 months)*

**Import from Revit, Archicad and Vectorworks** comes first (see "Leaving
another tool" below). Then issue management (BCF), clash detection, model comparison, linked modules,
revisions, details, automatic dimensioning, interior elevations, rendering,
sun studies…: the 25 V2 tools in [TOOLS.md](TOOLS.md).

## V3 *(from 2029)*

Several people working on the same project at once, MEP, structure, energy,
point clouds, Rhino…: the 13 V3 tools.

---

## Technical approach

**Our own building model, the best open components underneath.**

We do not adopt another product's model. The building model is ours and
thinks like an architect: stories, composite walls with junction priorities,
parametric elements built from parts (TOOLS.md, rule 15), renovation status
per layer, one view style. This is where the project's value lies, so we
write it ourselves.

Underneath, we reuse the best open libraries for what does not make the
difference: reading and writing IFC, geometry and Boolean operations, PDF and
DWG output, 3D display. Each one is picked on its merits, kept behind our own
interface so it can be replaced, and recorded in `PROVENANCE.md`.

The two-week prototype of Stage 0 builds three things on a candidate stack:
**stories**, **a composite wall with junction priorities** and **one
composable window**. It settles the stack (language, UI toolkit) and the
first libraries.

## Leaving another tool

A studio must be able to bring its projects in without losing its work. IFC
alone is not enough: types, composite walls, stories and views come out
damaged. We import the three main BIM formats, **Revit, Archicad and
Vectorworks**, in three steps:

1. **IFC, tested on real exports** of each tool (V1.0).
2. **Import add-ons** (V2): a small add-on runs inside Revit, Archicad or
   Vectorworks and sends the whole model to OPEN-ARCHIBIM through the tool's
   public programming interface. It is the richest and safest path. It needs
   a licence of the source tool at the time of the move.
3. **Direct readers** (V3): open an RVT, PLN or VWX file with no licence at
   all, for archives. Read only, built from the study of files, never from
   the software itself, and kept in a separate module.

We import, we do not write these formats: an open project lives in an open
format.

## Going to another tool

We do not write RVT, PLN or VWX. We write an IFC that every tool reads well,
which is a promise in itself:

- **IFC4 Reference View** by default: the export most tools read reliably,
  for coordination and handover. **IFC2x3 Coordination View 2.0** remains
  available for older tools and partners who still ask for it. IFC4.3 follows
  when the main tools read it.
- **Plain geometry first**: extrusions where they exist, closed solids
  otherwise, so that no tool has to guess.
- **Standard data**: buildingSMART property sets and quantities, stories,
  spaces, materials and layer sets, classifications through bSDD. Our own
  data goes into clearly named extra property sets, never in place of the
  standard ones.
- **Tested, not assumed**: every release exports a set of test projects,
  checks them with the buildingSMART validation service, and imports them
  into Revit, Archicad, Vectorworks and open viewers. What each tool loses is
  published in a compatibility table.

## Related projects

| Project | What it is | What we take from it |
|---|---|---|
| [BIMCraft](https://github.com/IPedrax/BIMCraft) | Open take on the Revit workflow, in Rust (MIT/Apache-2.0), drivable by agents (MCP) | Its "every action is a command" architecture gives undo, scripting and tests for free. Some of its libraries may be reused where its license allows. |
| [BimCraft (CadCraft)](https://github.com/getcadcraft/bimcraft) | Web BIM application in JavaScript (Apache-2.0), by a company; the names are trademarks | Its `PROVENANCE.md` is the model to follow: each engine states which public specification it comes from, and GPL tools are only used as test oracles. |
| [Bonsai / IfcOpenShell](https://ifcopenshell.org) | Reference open-source IFC kernel | The oracle that validates our IFC files, and a candidate IFC library. |

We learn from them and reuse their code when the license allows it, keeping
their notices (`NOTICE`). We take neither their names nor their Revit-shaped
logic.

---

## Funding

- **Claude for Open Source** (6 months of Claude Max for maintainers): apply
  once the repository has visibility, at the end of Stage 0 with the
  prototype video, or at the latest when V1.0 ships.
- **Open Collective / GitHub Sponsors**: transparent funding, public
  spending.
- **Partners**: pilot studios, architecture schools, professional bodies.

Each funded stage is announced with its budget and its deliverable.
