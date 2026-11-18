# Test cases

How the building model must behave, written by architects, in plain words.
These cases are the specification: the code is done when they pass. They do
not depend on the programming language, so they are written before the
code.

## Format

One YAML file per case, grouped by folder (`walls/`, `windows/`…):

```yaml
id: walls-l-corner-same-build-up      # unique, same as the file name
title: L corner between two identical walls
status: proposed                      # proposed | agreed
given: |
  The situation, with sizes and materials.
when: |
  The action, if any (a command the user runs).
expect:
  - One checkable result per line.
questions:                            # optional: what an architect must decide
  - An open question.
```

- `proposed`: written, not yet confirmed by an architect.
- `agreed`: confirmed by an architect maintainer. Only `agreed` cases
  become automated tests.
- A case with open `questions` cannot be `agreed`.

Sizes are in millimetres. Layers are listed from outside to inside.

## Contributing a case

Describe a real situation from your practice, in your own words. Never copy
a vendor's documentation: describe what an architect expects, not how a
product does it. Open a pull request with the `design` label.
