# 0004. Our own icon and symbol catalogue

Status: Accepted
Date: 2026-11-18

## Context

Architects find simple, monochrome pictograms clear. Icons of commercial
products are protected works and cannot be copied or traced.

## Decision

- We draw our own icon catalogue, as each tool is built: SVG on a
  24 × 24 grid, one stroke weight, a single colour that follows the theme.
- A short style guide in `assets/icons/` keeps every contributor's icons
  consistent.
- Generic actions (undo, save, zoom) may come from an open icon set with an
  MIT-compatible license, listed in PROVENANCE.md.
- Drawing symbols (door swings, window elevations) follow drawing
  conventions, which belong to no one, and are catalogued in
  `assets/symbols/`.
- No icon is copied or traced from another product.

## Consequences

- A consistent, recognisable visual identity that belongs to the project.
- Each new tool needs its icon: drawing it is part of the tool's
  definition of done.
