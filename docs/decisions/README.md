# Architecture decision records

One short page per structural choice: what we decided and why. Records are
never rewritten: to change a decision, write a new record that supersedes
the old one.

| # | Decision | Status |
|---|---|---|
| [0001](0001-rust.md) | Rust as the language | Accepted |
| [0002](0002-layered-workspace.md) | A layered Cargo workspace | Accepted |
| [0003](0003-gpu-incremental-rendering.md) | GPU rendering, recompute only what changed | Proposed |
| [0004](0004-own-icon-catalogue.md) | Our own icon and symbol catalogue | Accepted |
| [0005](0005-object-library.md) | Object library: parametric tools, presets and open content | Accepted |
| [0006](0006-constructive-build-ups.md) | Constructive build-ups, owned by each element | Accepted |

## Format

```markdown
# NNNN. Title

Status: Proposed | Accepted | Superseded by NNNN
Date: YYYY-MM-DD

## Context
What forces the decision.

## Decision
What we do.

## Consequences
What it costs, what it gives, what to watch.
```
