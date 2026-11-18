# Provenance

Where every engine, library and data set of OPEN-ARCHIBIM comes from. A
dependency that is not listed here does not ship.

Rules (see [CONTRIBUTING.md](CONTRIBUTING.md)):

- only licenses compatible with MIT are shipped;
- GPL tools may be used as test oracles, never shipped;
- each engine names the public specification it implements;
- notices of reused code are kept in `NOTICE`.

## Specifications

| Specification | Owner | Used for |
|---|---|---|
| IFC (ISO 16739) | buildingSMART | native format, import, export |
| BCF | buildingSMART | issue exchange (V2) |
| IDS | buildingSMART | information requirements (V3) |
| bSDD | buildingSMART | classifications and properties |

## Libraries

Chosen by the Stage 0 prototype (ADR 0007).

| Library | Version | License | Origin | Used for | Shipped |
|---|---|---|---|---|---|
| egui | 0.37 | MIT OR Apache-2.0 | crates.io, emilk/egui | widgets and the plan view (`archibim-ui`) | yes |
| eframe | 0.37 | MIT OR Apache-2.0 | crates.io, emilk/egui | the application window (`archibim-app`) | yes |
| wgpu | through eframe | MIT OR Apache-2.0 | crates.io, gfx-rs/wgpu | GPU drawing | yes |

These bring about 170 crates, all under permissive licenses (MIT, Apache-2.0,
BSD, ISC, Zlib, Unicode-3.0). The default fonts of egui are under the SIL
Open Font License and the Ubuntu Font Licence. One crate is offered under
"Apache-2.0 OR GPL-2.0-only": we use it under Apache-2.0. List them with
`cargo tree -p archibim-app -e normal -f "{p} {l}"`.

## Test oracles

| Tool | License | Used for |
|---|---|---|
| buildingSMART validation service | online service | checking exported IFC files |
