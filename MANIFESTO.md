# OPEN-ARCHIBIM Manifesto

> Architectural design software **for architects and by architects**.
> Open, lean, and at the service of the project, not the other way round.

## Why

Commercial BIM software has grown heavy. It piles up features that serve the
vendor's roadmap more than the profession. It locks projects into proprietary
formats and charges every year for access to our own work. Now it adds AI that
draws in our place instead of helping us draw better.

Meanwhile, basic gaps remain:

- wall junctions and material joints that have to be fixed by hand;
- 2D drawings that never come out of the model clean;
- IFC exchanges that lose information;
- a model that cannot be read, checked or reused outside the software.

OPEN-ARCHIBIM takes the best ideas of the tools architects already use,
keeps what works, simplifies it deeply, and writes it all anew. We lay down
again the foundations of what an architect really needs.

## Not another Revit clone

Almost every emerging open-source BIM project takes Revit as its model: an
engineering logic of families and templates, built for large firms and
multi-discipline teams. OPEN-ARCHIBIM goes the opposite way, the way of the
**architecture studio**:

- **the building is organised by stories.** You work floor by floor, the way
  you think about a project;
- **a wall is a composition of materials.** Junctions follow material
  priorities, not manual fixes;
- **2D is a first-class citizen.** You draw freely over the model, as on
  tracing paper, without having to model everything;
- **elements are parametric and built from parts.** One window, not a
  catalogue: add a sash, a transom or a side panel, change how each part
  opens, and every other property is kept;
- **the existing building is part of the project.** Renovation is the common
  case, not an option: every element is existing, to be demolished, or new;
- **you learn the tool in a week**, not in a three-month training course.

We look at what other open-source projects do, we learn from them, and we
reuse their code when their license allows it. But the way of working is the
architect's.

## Principles

### 1. The model is open

The native format is **IFC**, not a proprietary format exported to IFC
afterwards. Any compatible tool can read an OPEN-ARCHIBIM project, today and
in thirty years. Exchanges between project partners go through **BCF**.

### 2. The architect decides, the tool executes

The tool never makes a design decision in the architect's place. Automation
is welcome when it removes a repetitive task: junctions, dimensions,
schedules, updates.

The software can be **driven end to end**: every action is a command, exposed
by an **MCP** server. An AI can therefore work in the project with the same
commands as a person, and every command is visible and can be undone. AI is
an assistant you invite in. It stays **optional and transparent**, and the
software is complete without it.

### 3. Drawings are a deliverable

Plans, sections and elevations remain the language of the profession, with
clients, contractors and planning officers. They must come out of the model
**correct and legible, without retouching**: line weights, hatches, and
drawing conventions at every scale.

### 4. Less, but better

A feature enters the software only if practitioners need it on a real
project. An existing feature must be reliable before another one is added.
Ten flawless tools are better than a hundred approximate ones.

### 5. Your data belongs to you

Local first. No mandatory account, no telemetry by default, no subscription to
open your own project. The cloud, if it ever exists, stays the studio's
choice.

### 6. Clean-room design

No reverse engineering of proprietary software: no disassembly, no
decompilation, and no reuse of a vendor's code, internal formats or object
libraries. The project relies on **open standards** (IFC, BCF, bSDD), public
literature and architects' experience. Reading a product's public
documentation to understand a need is allowed. Copying its text, interface or
internal names is not: we describe the need in our own words. Contributions
that break this rule are rejected.

### 7. Governed by practitioners

Priorities are set with the people who design and deliver buildings:
architects, studios, schools, project managers. Developers build the tool;
architects say what it must do. Both talk in public, in the repository's
issues.

### 8. Open to every language

The project works in English so that anyone can contribute. The application
speaks the architect's language: every word on screen comes from a language
pack. The first packs are English, French, Spanish, Italian, German and
Portuguese.

## What we will not do

- One-click automatic building generation.
- A proprietary format, even "temporarily".
- Features locked behind a payment.
- A copy of another product's code, file formats, object libraries or
  interface text.

## Join the project

Architect, student, developer, teacher, translator, or studio ready to test:
open an issue describing **one concrete gap in the tool you use today**, with
a real case. That is the raw material of the roadmap.

See [ROADMAP.md](ROADMAP.md).
