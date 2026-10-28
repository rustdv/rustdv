# The shipped TinyALU testbench — Interlude and Chapter 40

Read [the repository instructions](../../AGENTS.md) for code,
verification, and release work. For manuscript edits, also follow the shared
[book instructions](../../book-pdf/AGENTS.md). These local notes add chapter-specific
teaching constraints; completed historical rewrites are not new tasks.

Manuscript: [interlude-tinyalu-testbench](../../book-pdf/src/interlude-tinyalu-testbench.md), [chapter-40-complete-tinyalu-testbench](../../book-pdf/src/chapter-40-complete-tinyalu-testbench.md).

## Preserve these teaching points

- The Interlude and Chapter 40 quote this same crate. The Interlude shows the
  shape; Chapter 40 explains the design choices. Source files are named after
  their concerns; the crate root is tinyalu_tb.rs.
- Preserve BFM-through-ConfigDb, factory-built children without service
  constructor arguments, subscriber-owned storage, sequence overrides,
  active/passive configuration, and scoreboard guards against checking nothing or
  leaving unmatched traffic.
- The TinyALU RTL self-clocks, and the BFM waits on edges. Use wait_idle as
  implemented rather than replacing completion with an assumed fixed number of
  clocks.
- Keep optional inspection, runtime FST capture, settled-service deadlines, and
  RTL coverage alongside the practical run guidance. Distinguish the functional
  Coverage collector and WITH_COVERAGE setting from simulator line/expression
  coverage.
- No Chapter 41 or replacement future/gaps chapter is queued. Keep limitations
  beside the mechanism that owns them.

Use [the current README](README.md) for the actual run output.
