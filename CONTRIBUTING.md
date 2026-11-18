# Contributing to OPEN-ARCHIBIM

Thank you for helping. OPEN-ARCHIBIM is built by architects and developers
together: architects say what the tool must do, developers build it. You do
not need to write code to contribute.

The project works in **English**: issues, code, comments, documentation and
commits. The application itself speaks every language through language packs
(see "Translating" below).

Read the [manifesto](MANIFESTO.md) first. It explains what we build and what
we refuse to build.

---

## 1. The clean-room rule

This rule protects the project. A contribution that breaks it is rejected,
however good it is.

**Allowed**

- Using other BIM tools as a user, and describing what works and what does
  not, **in your own words**.
- Reading a product's public documentation to understand a need.
- Open standards and their specifications: IFC, BCF, IDS, bSDD.
- Public literature, books, courses, your own professional experience.
- Studying files exported or saved by another tool, **only** to import a
  studio's own projects (see ROADMAP, "Leaving another tool").
- Reusing open-source code whose license allows it, keeping its notices.

**Not allowed**

- Disassembling, decompiling or debugging proprietary software, or using
  the results of someone who did.
- Copying text from a vendor's manual, help, interface, error messages or
  training material, even translated or paraphrased.
- Reusing a vendor's internal names, identifiers, file structures or
  object libraries (doors, windows, families, objects, templates).
- Using leaked, confidential or NDA-protected material: source code, SDK
  documentation under NDA, internal specifications.

If you have had access to a vendor's source code or confidential
documentation, tell the maintainers before contributing to the matching part
of the software.

Screenshots of another tool are welcome in an issue **to show a problem**.
They are never used as a design to copy.

### Sign-off

Every commit carries a sign-off line:

```
git commit -s -m "Add composite wall junction rule"
```

It adds `Signed-off-by: Your Name <your@email>` and certifies the
[Developer Certificate of Origin](https://developercertificate.org/): you
wrote the contribution or have the right to submit it. In this project it
also certifies that you followed the clean-room rule.

---

## 2. Architects: report a concrete gap

The roadmap is built from real cases. Open an issue with the
**"Concrete gap"** form and describe one problem you meet in the tool you use
today:

- what you were trying to do, on which kind of project;
- what the tool does, and how much time or quality it costs you;
- what you would expect instead, as an architect.

One gap per issue. Leave out client names, addresses and anything
confidential: describe the case, not the client.

You can also comment on [TOOLS.md](TOOLS.md): every tool in the review can
be discussed in the issues.

---

## 3. Design review by an architect

Any change that affects **how an architect works** gets the `design` label
and needs the approval of an architect maintainer before it is merged. That
covers tools, behaviour, the interface, drawing conventions, default
settings and the words on screen.

Purely technical changes (performance, refactoring, build, tests) are
reviewed by developers only.

When in doubt, add the label.

---

## 4. Developers: code contributions

The stack is chosen at the end of Stage 0 (see ROADMAP, "Technical
approach"). Until then, this section sets the rules that will not change.

- **Open an issue first** for anything larger than a fix, so that the need is
  agreed before the work starts.
- **One topic per pull request.** Small pull requests are reviewed faster.
- **Every action is a command.** A feature that changes the model goes
  through a command: it can be undone, scripted, tested and called through
  the MCP server.
- **No text in the code.** Every word shown to the user comes from a
  language pack key.
- **Tests come with the change.** A behaviour without a test is not
  finished.
- **New dependencies are recorded** in `PROVENANCE.md`, with their origin and
  license. Only licenses compatible with MIT are accepted in the shipped
  software. GPL tools may be used as test oracles, never shipped.
- **Commits** are written in English, in the imperative ("Add stair
  landing"), and signed off.

Coding style and the build guide will be added here with the stack.

---

## 5. Translating

The first language packs are **English, French, Spanish, Italian, German and
Portuguese**. Other languages are welcome.

- Translate **the profession's vocabulary**, not word for word: use the
  terms architects use in your country ("story", "étage", "planta", "piano",
  "Geschoss", "piso").
- Keep a **glossary** per language for the key terms, so that the same thing
  always has the same name.
- Respect **plural forms, number formats and units** of each language.
- Keep labels short: the interface has little room.
- Never copy a vendor's translation of its own interface. Use the words of
  the profession, which belong to no one.

The language pack format is chosen with the stack. Until then, help us build
the glossary in the issues, with the `translation` label.

---

## 6. Behaviour

Be kind, be precise, assume good faith. Criticise ideas, not people. The
maintainers may lock, edit or remove contributions that do not respect this.

## License

By contributing, you agree that your contribution is published under the
project's [MIT license](LICENSE).
