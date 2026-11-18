# 0002. A layered Cargo workspace

Status: Accepted
Date: 2026-11-18

## Context

The biggest source of technical debt in large applications is tangled
code: the interface changing data directly, file formats leaking into the
model, everything depending on everything.

## Decision

The code is a Cargo workspace of crates in layers (see
[ARCHITECTURE.md](../../ARCHITECTURE.md)). A crate depends only on the crates
below it. The building model depends on no other crate of the workspace.
Every change to the model goes through a command.

## Consequences

- The compiler enforces the boundaries: a forbidden dependency does not
  build.
- The model and the geometry can be tested without a window or a GPU.
- A library (IFC, geometry, interface) can be replaced inside one crate.
- Some data has to be passed explicitly between layers: more code at the
  boundaries, less confusion everywhere else.
