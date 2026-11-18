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

None yet. The Stage 0 prototype chooses the first ones.

| Library | Version | License | Origin | Used for | Shipped |
|---|---|---|---|---|---|

## Test oracles

| Tool | License | Used for |
|---|---|---|
| buildingSMART validation service | online service | checking exported IFC files |
