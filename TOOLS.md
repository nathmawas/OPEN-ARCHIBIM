# Tools: findings, rules and review

The tool review. Goal: **working software, fast**, without carrying over a
single useless setting. A tool comes in only if it serves a real project, and
it comes in its simplest form.

Review legend:

| | Meaning |
|---|---|
| **V1** | in the first usable version |
| **V2** | later, once V1 is reliable |
| **V3** | in the long run |
| **Dropped** | not carried over: we go through IFC to a dedicated tool, or another tool absorbs the feature |
| *merged* | the need is kept, in a simpler form, inside another tool |

---

## 1. Starting findings

These are frustrations studios live with in today's commercial BIM software.
The list stays open: propose each new finding as an issue.

**Business model and exchanges**
- The software is a lock-in, and very expensive.
- IFC exports are poor and too complex to configure.
- There is no real collaborative work.

**Editing**
- The eyedropper cannot be applied to several selected objects (hatches, for
  example): you have to go object by object.
- There is no default font: changing a font is done object by object.
- Repeated objects are not linked. Editing one instance does not change the
  others, and editing a favorite changes none of the objects already placed.
- Wall intersections and overlapping objects behave unpredictably.
- Doors and windows come as a catalogue of separate objects. Adding a glazed
  side panel to a door means starting again from another object and trying
  to carry the old properties over.

**Roofs**
- The reference plane is always the underside.
- Overhangs are hard to set, and each layer of the build-up cannot stop at its
  own limit.
- Non-planar roof slopes are impossible; you have to use a shell tool that is
  very hard to edit.

**Building tools**
- Railings and stairs are hard to use. A helical stair of more than 360° is
  almost impossible.
- Managing stories is tedious.

**Display**
- Pen settings are overwhelming: with so many parameters, nobody dares to
  change anything.
- Plan and 3D each have their own settings, often inconsistent. A tree that
  looks right in 3D does not display properly in plan or elevation.
- There are too many attributes. A material's settings are scattered across
  closed dialogs, with no live preview.
- You cannot work in plan, section and 3D at the same time in several windows.

**Libraries**
- A new library arrives every year: conflicts pile up and files get heavier.

**Measured on a real project** *(housing, tender documents stage, read
through the public API on 18 Nov 2026)*

| Setting | Count in the project |
|---|---|
| Layers | over 100 |
| Building materials | over 100 |
| 3D surfaces | over 100 (including *Metal - Gold*, *Paint - Peach*…) |
| Fill patterns | over 100 (including 19 numbered "Paving" patterns) |
| Composites | over 100, from two successive templates that overlap |
| Pen sets | 13, i.e. more than 3,000 adjustable pens |

The project also holds duplicates accumulated through copies (*Wood OSB 2*,
*PMR#1*, *… (1)*) and attributes unrelated to the project. This is the "too
many attributes" finding in numbers: rule 10 (nothing yearly, nothing
inherited) and rule 4 (one single material) answer it directly.

---

## 2. Design rules

Each finding becomes a rule that applies to **every** tool.

1. **A setting lives in one place.** A line weight or a hatch is set in a
   style. The object only points to that style, and changing the style
   changes the whole project. For text, the project defines **one font per
   purpose** (dimensions, annotations, areas, title block…), with the option
   to **unify** all fonts in one go. An object can always override locally.
2. **Type and instance.** Every placed object belongs to a type. Editing the
   type edits all its instances. An instance can override one parameter of
   its type, and the override stays visible and reversible. "Favorites"
   simply become types.
3. **Everything applies to the selection.** The eyedropper, parameter edits
   and style changes work the same on 1 or 500 objects. You choose which
   parameters to transfer.
4. **The material carries its appearance.** A material defines, in a single
   panel with a live preview:
   - its cut hatch and its surface hatch;
   - its 3D colour and texture;
   - its junction priority;
   - its physical properties.

   There is no other appearance setting.
5. **An object knows how to draw itself in every view.** Every object has a
   plan representation, a section or elevation representation, and a 3D one.
   They are defined together and previewed together. A tree has a symbol in
   plan, a silhouette in elevation and a volume in 3D.
6. **One display system.** A "view style" decides what you see and how, in
   plan, section or 3D. It sets:
   - the visible layers;
   - the level of detail;
   - the renovation filter;
   - graphic overrides.
7. **A few named line weights.** For example *extra fine, fine, medium, heavy,
   extra heavy*, with their value in mm per scale. No table of 255 pens.
8. **Several synchronised windows.** Plan, section and 3D open side by side.
   Select in one and the selection shows in the others.
9. **Native IFC, no translator.** There is nothing to configure to exchange:
   the saved file *is* IFC.
10. **No yearly library.** Objects are open files, versioned and embedded in
    the project. Updating the software never breaks a project.
11. **Every action is a command**, so it can be undone, scripted and driven
    through **MCP**. AI uses the same commands as a person: it does nothing an
    architect could not do and check.
12. **A command line.** Type the name of a tool or command and press Enter to
    run it, as in Rhino, or click. It is also how you search for a tool. These
    are the same commands as in rule 11.
13. **Renovate without demolishing the model.** External insulation is added
    as a new layer onto an existing wall, without deleting or redrawing the
    wall. Each layer carries its own status (existing, to be demolished,
    new).
14. **Every word on screen is translatable.** No text is hard-coded: every
    label, message and drawing annotation comes from a language pack. The
    first packs are English, French, Spanish, Italian, German and Portuguese.
    Units, number formats and drawing conventions follow the project's
    country, not the interface language.
15. **One parametric tool per element, built from parts.** There is no
    catalogue of door and window objects to choose from. There is one
    window, and you compose it: add a sash, a transom or a side panel,
    change how each part opens (side-hung, tilt-and-turn, sliding, fixed).
    Changing the composition never resets the other properties: a door that
    gains a glazed side panel keeps its sizes, materials, frame and data.
    The same applies to stairs, railings and curtain walls: the element
    stays the same, only its parts change.
16. **An element carries its architectural details.** What an architect
    draws as part of an element is an option of that element, not a separate
    object added against it. A wall can have a skirting or a cornice; a roof
    has its gutter, fascia and eaves; a stair has its railing and stringers;
    a window has its sill. Using one element to fake another (a thin wall as
    a skirting) is never needed, so it never creates clashes or junction
    problems.
17. **Standard sizes come ready to use.** Each parametric tool ships with
    presets taken from building practice and standards: common door blocks
    and their clear openings, accessibility widths, usual frame and leaf
    sections, window frame profiles. Presets are data, sorted by country, and
    reviewed by architects. The first presets are French, kept as close as
    possible to widely used models; other countries come from their own
    communities. A preset is a starting point: after it is
    placed, the element stays fully editable (rule 15).
18. **The build-up belongs to the element, described the way it is
    built.** There is no catalogue of fixed composites to pick from: a wall,
    a slab or a roof carries its build-up as its own parameters, started
    from a preset and edited in place (change a thickness without touching
    any shared setting). Each layer has a constructive role: load-bearing,
    insulation, air gap, membrane, lining, finish, covering. A layer can
    hold framing within its thickness (timber studs in the insulation),
    shown in plan. A system brings the layers it implies: tiles bring
    battens, zinc brings boarding, a timber roof brings rafters, purlins
    and trusses. A slab is one element whose floor finish can change from
    room to room. Renovation works layer by layer: the old insulation is
    marked for demolition and the new one appears as new work, in the same
    element, so an opening still crosses one single wall. Detail scales
    draw what the build-up implies (overlapping tiles, battens, fixings)
    without modelling it in 3D.

---

## 3. The review

Review settled by the architect on 18 Nov 2026, over 161 entries: the tools
and functions of Archicad 29, plus new features. One new entry was added
since: language packs. A feature marked *merged* is a V1 need, delivered in
the simpler form described by the rules in section 2. Quoted notes are the
architect's own comments.

### Design

| Tool | Review | Notes |
|---|---|---|
| Wall | **V1** | build-up as a parameter of the wall (rule 18) + junction priorities |
| Curtain Wall | **V1** | |
| Column | **V1** | |
| Beam | **V1** | |
| Slab | **V1** | |
| Roof | **V1** | redesigned: any reference plane, overhang per edge, independent layers |
| Non-planar roof | **V1** | new |
| Shell | **Dropped** | *"The shell becomes the roof; the two tools are unified"* — merged into Non-planar roof + Free form |
| Skylight | **V1** | its own tool, placed in a roof: roofer's work, not joinery |
| Dormer | **V1** | new: a parametric dormer placed on a roof (gable, hipped, shed, eyebrow…), with its cheeks, roof and front window; today it is either a rigid object or built by hand |
| Stair | **V1** | helical with no angle limit |
| Railing | **V1** | follows an edge, 3 components |
| Fence and gate | **V2** | new: fences, palisades, gates and their posts, along a line that follows the terrain; no more railings used as fences |
| Door | **V1** | one composable door (rule 15): leaves, side panels, fanlight |
| Window | **V1** | one composable window (rule 15): sashes, transom, opening type per part |
| Corner Window | **Dropped** | *"Same tool as windows"* — a window parameter |
| Wall End | **V2** | |
| Object | **V1** | linked component, 3 representations |
| Lamp | **V2** | |
| Zone | **V1** | |
| Mesh | **V1** | terrain from a land survey |
| Morph | **V1** | |
| Opening | **V1** | one tool for walls, slabs and roofs |
| TrussMaker | **V3** | |
| Solid Element Operations | **V1** | merged: Booleans in Free form |
| Library Part Maker | **V3** | |
| PARAM-O | **V3** | |

### Documentation

| Tool | Review | Notes |
|---|---|---|
| Linear dimension | **V1** | one Dimension tool, with modes |
| Level dimension | **V1** | a mode of the Dimension tool |
| Angle dimension | **V1** | a mode of the Dimension tool |
| Radial dimension | **V1** | a mode of the Dimension tool |
| Coordinate dimension | **V1** | |
| Automatic dimensioning | **V2** | |
| Text | **V1** | font per purpose |
| Label | **V1** | |
| Legend | **V2** | new: a layout legend built from the hatches, lines and symbols actually used in the placed drawings |
| Fill (hatch) | **V1** | |
| Line | **Dropped** | *"Line = polyline"* |
| Arc / Circle | **V1** | single "Draw" tool |
| Polyline | **V1** | single "Draw" tool |
| Spline | **V1** | single "Draw" tool |
| Hotspot | **V1** | merged: automatic snap points |
| Figure (image) | **V1** | background underlay |
| Camera | **V2** | |
| Section | **V1** | |
| Elevation | **V1** | a section without a cut plane |
| Interior Elevation | **V2** | |
| Worksheet | **V1** | |
| Detail | **V2** | |
| Change marker | **V1** | |
| Grid Element | **V1** | |
| Measure tool | **V1** | |
| Keynotes | **V1** | |
| Area list | **V1** | |

### Editing

| Tool | Review | Notes |
|---|---|---|
| Selection (arrow) | **V1** | |
| Marquee | **V1** | |
| Eyedropper / Syringe | **V1** | *"We will put a lot of work here to better control which properties can be copied"* — multi-selection, choice of parameters |
| Move, copy, multiply | **V1** | |
| Rotate, mirror | **V1** | |
| Stretch, resize | **V1** | |
| Adjust, trim, fillet | **V1** | |
| Offset, offset all | **V1** | |
| Explode | **V1** | |
| Group | **V1** | |
| Align, distribute | **V1** | |
| Reference overlay (Trace) | **V1** | at the heart of the drawing mode |
| Consolidate lines and fills | **V1** | |
| Linear overlap check | **V2** | |
| Guide lines, snap points | **V1** | |
| Origins | **V1** | |
| Coordinate input (tracker) | **V1** | |
| Multi-story editing | **V1** | |

### Attributes

| Tool | Review | Notes |
|---|---|---|
| Layers | **V1** | |
| Layer combinations | **V1** | merged: inside the view style |
| Line types | **V1** | small library |
| Fill patterns | **V1** | |
| Building materials | **V1** | *"Ideally materials and surfaces are set in the same place"* — one single material |
| Surfaces (3D) | **V1** | merged into Material |
| Composites | **V1** | merged: no catalogue of composites; the build-up is a parameter of each wall, slab and roof, started from presets (rule 18) |
| Complex profiles | **V1** | |
| Pen sets | **V1** | merged: named line weights |
| Attribute manager | **V2** | |
| Zone categories | **V1** | |
| Project fonts | **V1** | new |

### Display

| Tool | Review | Notes |
|---|---|---|
| Model view options | **V1** | merged: one view style |
| Graphic overrides | **V1** | merged: one view style |
| 3D styles | **V1** | merged: one view style |
| On-screen view options | **V1** | |
| Partial structure display | **V2** | |
| Renovation | **V1** | *"Here too there is a lot to do for renovation projects, for example adding external insulation without deleting the existing wall, just by adding a layer to it"* |
| 3D cutaway | **V1** | |
| Synchronised windows | **V1** | new |
| One view style | **V1** | new |

### Project

| Tool | Review | Notes |
|---|---|---|
| Country construction mode | **V3** | new: the project states the country it is built in, which sets presets, construction principles and drawing conventions; country communities maintain them |
| Stories | **V1** | |
| Working units | **V1** | |
| Location, survey point | **V1** | |
| Project preferences | **V1** | kept to a minimum |
| Scale | **V1** | |
| Work environment | **V1** | |
| Project templates | **V1** | |
| Favorites | **V1** | merged: they are types |
| Linked types and instances | **V1** | new |
| Library manager | **V1** | merged: embedded open components |
| Global library | **Dropped** | |
| Embedded library | **V1** | the only mode |
| GDL object language | **Dropped** | open format |
| Properties | **V1** | |
| Classifications | **V1** | minimal |
| Calculated properties (expressions) | **V1** | |
| ID management | **V1** | |
| Design options | **V2** | |
| Element information | **V1** | |
| Navigator, project map | **V1** | |
| Saved views | **V1** | |

### Production

| Tool | Review | Notes |
|---|---|---|
| Layouts | **V1** | |
| Master layouts | **V1** | |
| Placed drawings | **V1** | |
| Sheet numbering | **V1** | |
| Revision management | **V2** | |
| Publishing | **V1** | simple |
| Interactive schedules | **V1** | simple |
| Surface schedules | **V2** | |
| Lists (legacy calculation) | **Dropped** | |

### Collaboration

| Tool | Review | Notes |
|---|---|---|
| Teamwork / BIMcloud | **V3** | |
| Hotlinked modules | **V2** | external linked component |
| Clash detection | **V2** | |
| Model compare | **V2** | |
| Issue management (BCF) | **V2** | |
| Change tracking | **V2** | |
| Bimplus connection | **Dropped** | |
| Bluebeam connection | **Dropped** | |
| Solibri connection | **Dropped** | through IFC |
| BIMx | **Dropped** | IFC + an open viewer |

### Exchanges

| Tool | Review | Notes |
|---|---|---|
| IFC (open, save) | **V1** | native |
| IFC translators | **Dropped** | nothing to configure |
| Merge, hotlink an IFC | **V1** | as a reference |
| IDS import | **V3** | |
| DWG / DXF | **V1** | |
| DWG translators | **Dropped** | minimal settings |
| XREF | **V2** | |
| PDF | **V1** | |
| Point clouds | **V2** | |
| Rhino 3D | **V2** | |
| RFA / RVT | **V3** | direct reading; import through an add-on comes in V2 (ROADMAP, "Leaving another tool") |
| FBX, OBJ | **V2** | |
| SAF | **V3** | |
| FRILO, Artlantis | **V3** | |
| Spreadsheet (properties) | **V2** | |
| MCP server | **V1** | new |

### Visualisation

| Tool | Review | Notes |
|---|---|---|
| Photorealistic rendering | **V2** | |
| Sun study | **V2** | |
| Visualisation extras | **Dropped** | |

### Engineering

| Tool | Review | Notes |
|---|---|---|
| MEP Designer | **V3** | |
| Structural analytical model | **V3** | |
| Energy evaluation | **V3** | |
| Environmental properties | **V3** | |

### Interface

| Tool | Review | Notes |
|---|---|---|
| Palettes and toolbars | **V1** | simplified |
| Dialog customisation | **Dropped** | one live panel |
| Keyboard shortcuts | **V1** | *"Use a command mode like Archicad or Rhino: type the command and press Enter to start the tool (or click), which also lets you search for tools"* |
| Add-ons | **V2** | through commands |
| Goodies | **V3** | |
| Backup, archives | **V1** | |
| File compression | **Dropped** | useless with an open format |
| Bug reporter | **V1** | |
| Language packs (EN, FR, ES, IT, DE, PT) | **V1** | new — rule 14 |

---

## 4. V1 stages

V1 holds 110 tools. It ships in five stages. Each stage ends with something an
architect can try on a real project. The scope does not change, only the
delivery order. A merged feature arrives with the tool that absorbs it.

| Stage | Name | What you can do at the end | Tools |
|---|---|---|---|
| **V1.0** | The core | A simple house (without a roof) drawn in plan and 3D, saved and read back as IFC. | 39 |
| **V1.1** | The complete building | The same house with roof, stairs, railings, furniture, terrain and rooms, as new build or renovation. | 15 |
| **V1.2** | The drawing | Plans, sections and elevations legible without retouching: lines, hatches, texts and dimensions set once for the whole project. | 30 |
| **V1.3** | The permit set | A complete building permit application, published as PDF and DWG. | 21 |
| **V1.4** | Free forms | Warped roofs, sculpted volumes, curtain walls, drawn profiles. | 5 |

### V1.0 — The core

A simple house (without a roof) drawn in plan and 3D, saved and read back as
IFC.

Stories · Working units · Scale · Origins · Grid Element · Wall · Composites ·
Building materials · Surfaces (3D) · Slab · Door · Window · Opening · Column ·
Beam · Selection (arrow) · Marquee · Move, copy, multiply · Rotate, mirror ·
Stretch, resize · Adjust, trim, fillet · Offset, offset all · Guide lines,
snap points · Coordinate input (tracker) · Measure tool · Layers · Linked
types and instances · Element information · Navigator, project map · Palettes
and toolbars · Keyboard shortcuts · Synchronised windows · On-screen view
options · Project preferences · IFC (open, save) · Backup, archives · MCP
server · Bug reporter · Language packs *(infrastructure, with English and
French)*

### V1.1 — The complete building

The same house with roof, stairs, railings, furniture, terrain and rooms, as
new build or renovation.

Roof · Dormer · Stair · Railing · Skylight · Object · Embedded library · Library
manager · Favorites · Mesh · Location, survey point · Zone · Zone categories ·
Renovation · Multi-story editing

### V1.2 — The drawing

Plans, sections and elevations legible without retouching: lines, hatches,
texts and dimensions set once for the whole project.

One view style · Model view options · Graphic overrides · 3D styles · Layer
combinations · Pen sets · Line types · Fill patterns · Project fonts · Text ·
Label · Fill (hatch) · Arc / Circle · Polyline · Spline · Linear dimension ·
Level dimension · Angle dimension · Radial dimension · Section · Elevation ·
3D cutaway · Figure (image) · Reference overlay (Trace) · Eyedropper /
Syringe · Group · Align, distribute · Explode · Consolidate lines and fills ·
Hotspot

### V1.3 — The permit set

A complete building permit application, published as PDF and DWG. The
Spanish, Italian, German and Portuguese language packs complete the first six
languages.

Layouts · Master layouts · Placed drawings · Sheet numbering · Saved views ·
Publishing · PDF · DWG / DXF · Merge, hotlink an IFC · Interactive schedules ·
Area list · Properties · Classifications · Calculated properties
(expressions) · ID management · Project templates · Keynotes · Change marker ·
Worksheet · Coordinate dimension · Work environment

### V1.4 — Free forms

Warped roofs, sculpted volumes, curtain walls, drawn profiles.

Non-planar roof · Morph · Solid Element Operations · Curtain Wall · Complex
profiles

---

## Summary

| | Count |
|---|---|
| V1 | 110 |
| V2 | 27 |
| V3 | 14 |
| Dropped | 15 |
| **Total** | **166** |

To be discussed in the issues, tool by tool.
