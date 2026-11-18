# CLAUDE.md

Instructions for AI assistants (and a summary for humans) working in this
repository.

## The project

OPEN-ARCHIBIM is open-source BIM software for architecture studios, MIT
licensed. Read [MANIFESTO.md](MANIFESTO.md), [ROADMAP.md](ROADMAP.md) and the
design rules in [TOOLS.md](TOOLS.md) section 2 before any design work.

The maintainer is an architect. Architects decide what the tool does;
code serves that decision. When a behaviour is not settled by a rule or a
test case, ask, do not guess.

## Language

- Everything in the repository is in **English**: code, comments, docs,
  issues, commits.
- No user-facing text in the code. Every word on screen comes from a language
  pack key (TOOLS.md, rule 14).

## Clean-room rule (non-negotiable)

Follow [CONTRIBUTING.md](CONTRIBUTING.md), section 1. In short:

- never write code, names, file structures or text taken from proprietary
  software, its manuals, help or interface, even paraphrased or translated;
- never use results of disassembly, decompilation or debugging of
  proprietary software;
- build from open standards (IFC, BCF, IDS, bSDD), public knowledge and the
  test cases in `tests/cases/`.

If a request would break this rule, stop and say so.

## How we work

- **Test cases first.** Architect behaviour is written in `tests/cases/` as
  plain YAML (see its README). Code is done when the matching cases pass.
- **Every action is a command**: undoable, scriptable, testable, exposed
  through the MCP server.
- **Small pull requests**, one topic each, from a branch, never on `main`.
- **Commits** are in English, in the imperative, and signed off
  (`git commit -s`).
- **New dependencies** go into [PROVENANCE.md](PROVENANCE.md) with origin and
  license. Only MIT-compatible licenses ship; GPL tools are test oracles
  only.
- **Explain changes** in plain words in the pull request: the maintainer is
  learning software development and reviews every change.

## Architecture and code quality

Read [ARCHITECTURE.md](ARCHITECTURE.md) and the records in
[docs/decisions](docs/decisions/README.md) before writing code.

- **Rust**, one Cargo workspace, crates in layers. A crate depends only on
  the crates below it; `archibim-model` depends on no other crate.
- Nothing changes the model outside a command. A command returns the
  elements it changed.
- Only what changed is recomputed; heavy work never runs on the interface
  thread. Respect the performance budget in ARCHITECTURE.md.
- `rustfmt`, `clippy` with no warnings, no `unwrap()`/`expect()` in
  library crates, no `unsafe` outside `archibim-render`.
- Technical debt is written down: a `tech-debt` issue and a
  `TODO(#issue)` comment, never a silent shortcut. Flag any debt you see.
- A structural change needs a new decision record in `docs/decisions`.
- Each new tool comes with its own icon, drawn to the catalogue rules
  (ADR 0004). Never copy or trace another product's icons.

Build, test and benchmark commands will be added here with the first code.
