# Book instructions

Read and follow [the repository instructions](../AGENTS.md) first. This file
adds the rules for *Rust for RTL Verification*. It is the single book-level
instruction source for shared rules; [chapter-specific guidance](#chapter-specific-guidance)
links to teaching notes in the runnable examples’ READMEs and separate local
instructions.

## Scope and current state

The manuscript has 40 chapters, an Interlude, the unnumbered Toolkit page,
and Appendices A–D. [src/SUMMARY.md](src/SUMMARY.md) defines their order. The
original prose pass, caption reconciliation, transcript completion, child
attribute sweep, and final read are complete. Work now is the requested
revision; do not restart those passes or resurrect their finished task lists.

- For a prose-only task, edit the manuscript and relevant book notes. Do not
  change Rust, HDL, scripts, or manifests to make an explanation true. If the
  task also authorizes code changes, follow the parent instructions and update
  every quoted artifact in the same session.
- The implementation and runnable examples decide technical facts. Read them
  before relying on an old planning note. If an example appears wrong or a
  proposed figure has no runnable source, report the gap rather than invent
  evidence or silently change the implementation.
- Do not change a finished chapter beyond the requested scope merely because
  a different presentation appeals to you. Revise a false premise when the
  task requires it; preserve useful explanations and the book's voice.
- Keep chapter-specific teaching constraints in the linked README sections;
  record active questions and validation in the repository's STATUS.md. Do not
  create another handoff, restart prompt, or parallel progress register.
- Honor existing user authorization. Ask about genuinely unresolved editorial
  choices or facts marked ASK RAY; do not treat old, resolved questions or the
  completed initial voice checkpoint as reasons to pause routine revisions.

## Reader, argument, and voice

The reader is a verification engineer familiar with SystemVerilog UVM or
Python. Assume reading ability in both languages, but no Rust experience, no
earlier book, and no prior use of cocotb or pyuvm. Explain each mechanism from
that starting point.

### Do not sell types, and do not disparage what came before

The subject is verification. State the frame once in Chapter 1: types for
data and ownership; runtime indirection for topology and binding. The ConfigDb,
factory, and TLM deliberately resolve at run time. Never promise that a typo
in configuration or a missing connection is a compile error. Real compiler
examples concern data, ownership, and the constraints the source demonstrates.

Explain the present design directly. Do not tell the reader about retired
framework APIs, earlier drafts, or internal design arguments. The reader's
own testbench progression from 1.0 through 8.0 is part of the teaching arc and
can be compared. Costs and limitations belong beside the mechanism they limit.

Comparisons orient rather than score. Use both source languages where useful,
SystemVerilog first, for example *(SV: `uvm_config_db#(int)::set`; pyuvm:
`ConfigDB().set`)*. Never assume the reader remembers an earlier book. Do not
mock SystemVerilog, Python, UVM, cocotb, pyuvm, or their users, and do not
celebrate the compiler or the absence of a type system.

Use short, present-tense declarative sentences and paragraphs of two to four
sentences. "We" does the work with the reader; "you" gives instructions;
"I" marks an author's choice or experience. Avoid sales language and padding
such as "honestly", "genuinely", "straightforward", "promise kept", or "the
seam at work". Dry jokes belong sparingly in parentheses, footnotes, or example
names, never in the technical argument.

## Teaching pattern and exemplars

The model is *Python for RTL Verification* in
`../rustdv-reference/salemi_books/` (paths here are from the repository root).
The tone study is already done; retain its working method:

1. Open a chapter by locating it in the testbench arc and stating the problem
   before naming its solution. Keep headings short and concrete: a component,
   mechanism, or task. Longer storytelling belongs in Chapter 1, not in the
   middle of a mechanism explanation.
2. Introduce identifiers before the first listing that uses them. Announce
   what a figure shows in one or two sentences, then explain only its new
   lines, in source order. A bulleted walkthrough suits new code; a numbered
   comparison suits changes between the reader's testbench versions.
3. Refer back by chapter number instead of reteaching known material. A
   forward question may connect sections when the next section answers it.
4. Put real output after the example it demonstrates and interpret a specific
   line. End with a short Summary of the mechanism and, when useful, one
   concrete pointer to what the next chapter needs.

For a substantial prose revision, use [Chapter 5](src/chapter-05-ownership.md)
as the exemplar for a surgical Part I edit and
[Chapter 23](src/chapter-23-uvm-test-testbench-3.0.md) for a testbench chapter's
opening, walkthrough, transcript, and closing. Read the earlier book only when
needed to resolve a voice question; do not repeat a completed tone study.

Read a chapter’s README teaching notes, any local instructions, current
manuscript, and source crate before editing it. Avoid reading the manuscript in bulk for a narrow correction.
Finish at a coherent chapter boundary and keep cross-chapter promises intact.

## Sources and terminology

- Chapter examples live in `output/examples/`. The Interlude and Chapter 40
  use the same shipped `rustdv/tinyalu_tb` crate and its README transcript.
  Use the actual named crate roots, not an assumed `src/lib.rs`.
- Read only the relevant upstream sources in `../rustdv-reference/` when a
  claim about UVM, cocotb, or pyuvm needs checking. They are read-only. Internal
  decision history is not required reading for ordinary prose work and must
  not appear as D-number citations in the manuscript.
- SystemVerilog quotations come from
  `../rustdv-reference/uvmprimer-master/`, carry a source label, run at most
  15 lines, and appear only a handful of times across the book. Do not pair a
  quotation with invented simulator output.
- Published prose should not depend on incidental repository layout. Say
  "the book's repository" when that is sufficient; project-layout figures and
  executable commands must use verified paths where paths are necessary.
- SUMMARY keeps its bare separators. Name chapter numbers in any Part I/II
  reference so it resolves without rendered part labels. There is no
  Chapter 41; limitations belong beside the relevant mechanisms.
- Use bare `#[component]`; the field's Rust type determines its role. The
  factory registers components through the derive, and constructors do not
  receive framework services that arrive through the ConfigDb.
- A `Subscriber` is a plain object implementing `write()`; a component hosts
  it. Parents `connect` streams; components `subscribe` receivers. The
  `AnalysisBus` stores no transactions. FIFO `put_ap()`/`get_ap()` taps observe
  traffic without consuming or delaying it.
- Preserve the learning arc: the Interlude shows the destination without
  explaining everything; Chapters 15–20 support useful component-free
  testbenches; Chapter 40 returns to the Interlude's crate with the mechanisms
  explained. The TinyALU supplies its own clock; Chapter 17 teaches `Clock`
  for the counter that needs a testbench-driven clock.

## Chapter-specific guidance

The manuscript chapters are flat files in `src/`; their local teaching notes
are in the runnable examples’ READMEs. Read the relevant linked teaching
section and any local `AGENTS.md` before editing that example or its book
passage. Follow repository rules for code work; these notes do not turn it
into a prose-only task.

| Chapter | Teaching notes / local instructions |
|---|---|
| 1 | [ch01-why-rust](../output/examples/ch01-why-rust/README.md#preserve-these-teaching-points) |
| 5 | [ch05-ownership](../output/examples/ch05-ownership/README.md#preserve-these-teaching-points) |
| 13 | [ch13-smart-pointers](../output/examples/ch13-smart-pointers/README.md#preserve-these-teaching-points) |
| 15 | [ch15-async-await-executor](../output/examples/ch15-async-await-executor/README.md#preserve-these-teaching-points) |
| 17 | [ch17-simulating-with-rustdv-sim](../output/examples/ch17-simulating-with-rustdv-sim/README.md#preserve-these-teaching-points) |
| 19 | [ch19-tinyalubfm](../output/examples/ch19-tinyalubfm/README.md#preserve-these-teaching-points) |
| 20 | [ch20-struct-based-testbench-2.0](../output/examples/ch20-struct-based-testbench-2.0/README.md#preserve-these-teaching-points) |
| 21 | [ch21-macros](../output/examples/ch21-macros/README.md#preserve-these-teaching-points) |
| 23 | [ch23-uvm-test-testbench-3.0](../output/examples/ch23-uvm-test-testbench-3.0/README.md#preserve-these-teaching-points) |
| 24 | [ch24-components](../output/examples/ch24-components/README.md#preserve-these-teaching-points) |
| 25 | [ch25-uvm-env-testbench-4.0](../output/examples/ch25-uvm-env-testbench-4.0/README.md#preserve-these-teaching-points) |
| 26 | [ch26-logging](../output/examples/ch26-logging/README.md#preserve-these-teaching-points) |
| 27 | [ch27-configuration](../output/examples/ch27-configuration/README.md#preserve-these-teaching-points) |
| 28 | [ch28-config-debugging](../output/examples/ch28-config-debugging/README.md#preserve-these-teaching-points) |
| 29 | [ch29-factory](../output/examples/ch29-factory/README.md#preserve-these-teaching-points) |
| 30 | [ch30-variation-point-testbench-5.0](../output/examples/ch30-variation-point-testbench-5.0/README.md#preserve-these-teaching-points) |
| 31 | [ch31-component-communications](../output/examples/ch31-component-communications/README.md#preserve-these-teaching-points) |
| 32 | [ch32-analysis-ports](../output/examples/ch32-analysis-ports/README.md#preserve-these-teaching-points) |
| 33–34 | [ch34-connections-testbench-6.0](../output/examples/ch34-connections-testbench-6.0/README.md#preserve-these-teaching-points) |
| 35 | [ch35-transactions](../output/examples/ch35-transactions/README.md#preserve-these-teaching-points) |
| 36 | [ch36-sequence-testbench-7.0](../output/examples/ch36-sequence-testbench-7.0/README.md#preserve-these-teaching-points) |
| 37 | [ch37-out-of-order-transaction-testbench-7.1](../output/examples/ch37-out-of-order-transaction-testbench-7.1/README.md#preserve-these-teaching-points) |
| 38 | [ch38-fibonacci-testbench-7.2](../output/examples/ch38-fibonacci-testbench-7.2/README.md#preserve-these-teaching-points) |
| 39 | [ch39-virtual-sequence-testbench-8.0](../output/examples/ch39-virtual-sequence-testbench-8.0/README.md#preserve-these-teaching-points) |
| 40 | [tinyalu_tb](../rustdv/tinyalu_tb/AGENTS.md) |

Chapters without separate teaching notes use the shared rules and their
current source and README. Chapters 1–14 teach Rust first; keep their fixed source listings,
introduce language concepts before later framework uses, and retain the dual
SystemVerilog/Python audience. Chapter 22 is prose-only and uses the book’s
runtime-binding argument, not an example crate. Chapter 33’s local folder
[points to its shared Chapter 34 source](../output/examples/ch33-components-testbench-6.0/AGENTS.md).

Appendix A maps chapter numbering to the earlier books (Chapter 37 is out of
order; Chapter 38 is Fibonacci). Appendices B/C translate actual APIs;
Appendix D and the Toolkit follow the real exports and import paths. The
latest feature audit is recorded in STATUS.md; do not duplicate a dated
per-chapter completion register here.

## The Toolkit and API reference

The unnumbered Toolkit sits between the Interlude and Chapter 15. It declares
the names the prelude otherwise hides; Appendix D is the alphabetical
reference. Introduce framework identifiers there or in an earlier chapter
before using them in a listing. Recheck the chapter column against actual
teaching, not an old first-use guess.

On every release, follow the parent release audit. Compare the real prelude
and facade exports with the Toolkit and Appendix D, including explicit trait
imports and APIs outside the prelude. Review Chapter 17's signal/scheduler
explanation, Chapter 40's simulator/trace/coverage guidance, and Appendices B/C.
Existing listings can pass while new capabilities remain undocumented. Keep
optional advanced services beside their prerequisites, distinguish functional
from RTL coverage, and state simulator and host/build requirements.

## Listings, figures, and transcripts

- Chapters 1–14 are compared byte-for-byte with their examples by book-sync.
  A prose edit leaves those listings untouched. A task that explicitly
  changes an example must update the source and quoted text together under
  the parent instructions.
- Chapters 15–40, the Interlude, and the repository README are checked against
  their source crates by `verify-book-listings.py`. Use the current source,
  not a stale manuscript listing. Do not add drift exemptions to pass a check.
- Copy transcripts verbatim from the relevant example README or
  `rustdv/tinyalu_tb/README.md`; contiguous excerpts are allowed. Never
  compose, retype, tidy, or infer output. If a prose-only task encounters a
  missing or questionable transcript, record the exact source/run needed and
  report it rather than inventing a block. Simulator reruns and regeneration
  belong to tasks whose scope includes that verification.
- One sequence per chapter numbers code, drawings, tables, and output as
  Figures in appearance order. Existing numbering is settled. Preserve it
  for ordinary revisions; if an authorized addition moves figures, reconcile
  manuscript captions, crate captions, README maps, and references together.
- Small structural drawings use `text` fences with a `# Figure N:` caption.
  Dataflow/architecture drawings use inline SVG in `<figure>`, with an italic
  `<figcaption>`. Use stroke `#888`, fill `none`, and text `currentColor` so
  both mdBook themes remain readable.
- Source-caption edits must be in place and line-count-neutral unless the
  task also regenerates all affected transcripts. Output embeds `file:line`;
  rename files before rerunning, not afterward. Chapters 1–14 figure IDs also
  feed `output/examples/manifest.json` and book-sync; an authorized renumbering
  must keep those aligned. Current maps are the source captions and READMEs.

## Applied caption reconciliation (historical)

The caption reconciliation from the original prose pass is complete. This
compact record preserves the old crate-to-book mappings; **do not replay
them against current captions**. Current source/README maps take precedence,
including later additions and example replacements.

| Chapter | Applied mapping or constraint |
|---|---|
| 1–14 | No caption changes; figure IDs remain tied to manifest/book-sync. |
| 24 | No numbers moved. Preserve the deliberate source gaps filled by output and the book-only tree drawing. |
| 27 | Old crate 4, 5, 6, 7, 8, 9 became book 5, 6, 8, 9, 11, 12; transcript slots were 4, 7, 10, 13. |
| 28 | Old crate 7, 8, 9 became book 8, 9, 11; output occupied 7, 10, 12. |
| 31 | Old crate 4, 5, 6, 7, 8, 9, 10, 11, 12 became book 5, 6, 7, 9, 10, 11, 12, 14, 16. Subsequent FIFO-tap placement belongs to Chapter 32. |
| 32 | Old crate 5, 6, 7 became book 6, 8, 9. The later tap example has its own current map in the Chapter 32 README. |
| 33–34 | Chapter 33 component captions in the shared source stayed unchanged. Old Chapter 34 captions 1, 2 became 2, 3, following its architecture drawing. |
| 36 | Old crate 1–7 became book 2–8 after the handshake drawing; existing uncaptioned transaction re-shows stayed uncaptioned. |
| 37 | The original repair-desk map is obsolete; the replacement out-of-order example's README is authoritative. |
| 39 | Old crate 1, 2, 3, 4, 5, 6 became book 1, 4, 6, 7, 8, 2; uncaptioned helper/test listings stayed uncaptioned. |
| Interlude, Toolkit, 15–23, 25–26, 29–30, 35, 38, 40 | No other source-caption numbers moved in that pass. Book-only figures and quoted excerpts retain their own current maps. |

File renames preceded transcript regeneration, source-caption edits were
line-count-neutral, and transcript completion was part of the finished pass.
These are records of completed work, not a list of outstanding rewrites.

## Validation and close-out

From the repository root, run checks appropriate to the edit:

```sh
python3 output/regression/regress.py --suite book-sync
python3 output/regression/verify-book-listings.py
python3 output/regression/verify-no-stale-versions.py
```

For changes to examples or quoted output, follow the parent verification
rules, including `bash output/regression/verify-transcripts.sh`. A prose-only
edit can verify that existing code/output blocks and captions are unchanged;
do not rerun the full simulator suite just to consolidate instructions.

Build the book after changing its structure or substantial prose, and review
the affected rendered pages for references, fences, figures, and drawings:

```sh
mdbook build book-pdf --dest-dir /tmp/rustdv-$(id -u)/book-review
```

Use a locally available mdBook binary or obtain the appropriate official
build when needed; do not assume the old offline toolchain drop exists.
`mdbook-pdf` is optional and needs Chrome/Chromium. A missing optional PDF
renderer permits an HTML build; report that limitation rather than claiming
the PDF was verified. Never edit generated HTML/PDF output.

For a release, the PDF rebuild is required: use the fresh-build procedure in
[README.md](README.md), verify that the PDF backend produced a new file, review
the affected pages, and replace `../Rust4RTLVerif.pdf`. Include that PDF in the
same release commit as its manuscript sources. An HTML-only build is useful
for ordinary edits but does not satisfy the release PDF step.

Update affected README teaching notes, local instructions, and STATUS.md with
actual changes, validation, remaining questions, and the next concrete action
if work remains. Keep
completed items marked complete, preserve earlier user edits, and follow the
requested commit/push state. Do not name a working branch in documentation.
