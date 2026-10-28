# Chapter 21: Macros: Code That Writes Code — figure map

Pure-Rust and Python figures; no simulator needed.

| Figure | Title | Where |
|---|---|---|
| 1 | What @cocotb.test() really does — a registering decorator | `python/ch21_fig01_a_registering_decorator.py` (run with python3) |
| 2 | A declarative macro — patterns in, code out | `src/bin/ch21_fig02_a_declarative_macro.rs` |
| 3 | What #[rustdv::test] expands to (tidied) | fragment — tidied from `rustdv/rustdv-macros/src/rustdv_macros.rs` |
| 4 | Link-time registration in miniature | `src/bin/ch21_fig04_link_time_registration_in_miniature.rs` |
| 5 | What #[derive(Component)] writes for you (tidied) | fragment — tidied from `rustdv/rustdv-macros/src/rustdv_macros.rs` |

## Preserve these teaching points

Manuscript: [chapter-21-macros](../../../book-pdf/src/chapter-21-macros.md).

- The source examples are bins under src/bin; this package is a macro
  demonstration without a simulator test and remains outside the simulator chapter
  runs.
- Test/component registration uses linkme distributed slices; procedural macros
  parse and emit Rust with syn/quote. Do not restore handwritten token-walking or
  linker-boundary explanations as the implementation.
- Teach bare #[component]. The field type determines its role, and deriving
  Component supplies by-name factory registration where supported; no extra
  registration attribute is needed.

Read this crate’s source and this README before changing a quoted
figure or behavioral claim; keep the source, figure map, and transcript aligned.
