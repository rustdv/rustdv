# Project: rustdv & "Rust for RTL Verification"

**Write nothing he cannot act on.** Steps are fine — say what you are doing as
you go if it helps. Paragraphs of information that require no action are the
problem, and they are what has cost this project the most context.

- **Every paragraph is a step, a result, or something he must do or decide.**
  Nothing else. Cut background he did not ask for, justification of a choice
  already made, his own points restated, and analysis of your mistakes once they
  are fixed.
- **Never write "worth your attention", "worth noting", or "two things to
  flag".** You do not know what is worth his attention. He reads a heading like
  that as a question, finds none in it, and has spent his time for nothing.
- **If he has to do something, say it plainly. If he does not, say that in one
  line and stop.**
- **Fix what you find; don't report it.** A finding that changes nothing for him
  goes in `STATUS.md` or `output/.design-decisions.md`, where the next thread
  will read it.

## Context
rustdv is a Rust hardware verification framework (a cocotb + pyuvm analog) by
Ray Salemi, companion to the book *Rust for RTL Verification*. The repo holds
two products (see README.md for the reader-facing overview):

1. **rustdv** — the framework (crates in /rustdv), with a TinyALU regression
   passing on Icarus Verilog.
2. **"Rust for RTL Verification"** — chapters 1–40 plus an Interlude and four
   appendices in /book-pdf/src (mdBook), every figure verified running.

**Both products are complete.** The framework's UVM restoration is finished —
phases, the ConfigDb, the factory and the whole TLM layer are the settled
design, not work in progress — and the manuscript is written, with every
transcript real simulator output and every ch15–40 listing checked against its
crate. Ongoing work is revision, not construction.

**Never write a branch name into a document.** Branches are ephemeral; a branch
named in a file is stale within days and has repeatedly sent threads to the
wrong place. Run `git status` and `git branch --show-current` if you need to
know where you are. Nothing in this repository's prose should answer that
question.

`output/.design-decisions.md` is the authoritative decision log — read its
§0 (the mission and method) and *.local.md before proposing anything
architectural. The decisions that most shape new code: **D83b** (connection is
a trait method, not a registry — if a mechanism works for a child but needs a
second spelling for `self`, it has broken the UVM's uniformity), **D82b/D82c**
(children move out of the parent for the run phase, and each component races
the objection event individually, never the whole tree), **D90** (the analysis
hub stores nothing — the subscriber owns its storage), and **D3** (where a
statically-typed language with a static option in hand still chose runtime
indirection, the indirection is load-bearing; resist the urge to make late
binding static, which is the error this project exists to undo).

Read this file first for repository work. README.md is the reader-facing
orientation; STATUS.md records completed work and validation, newest last.

Ground truth for the methodology — the cocotb, pyuvm and SystemVerilog UVM
sources, plus the example code from the earlier books — lives outside this
repository at ../rustdv-reference, so the repo carries only its own product.

## Folder map
- /rustdv — the framework crates (rustdv-gpi-sys … rustdv, plus
  tinyalu_tb). Runs via sim/run_rustdv.sh.
- /rustdv/README.md — getting started, typed signal/variable access, and API
  migration. /sim/README.md covers simulator modes, runtime capture, and RTL
  coverage. The Cargo workspace root is the repository root.
- /book-pdf/src — the manuscript; /book-pdf/AGENTS.md holds its editorial
  rules and links to chapter README teaching notes and local instructions.
  Rebuild with mdBook.
- /output/examples — every book figure, runnable (Part I per-figure files,
  Parts II–V as sim chapter crates). Read a chapter’s README teaching notes
  and any local AGENTS.md before editing its examples; the index is in
  /book-pdf/AGENTS.md. The shipped /rustdv/tinyalu_tb/AGENTS.md covers the
  Interlude and Chapter 40.
- /output/regression — regress.py; TESTING.md explains the suites and installing
  the git pre-push hook.
- /skills — AI verification skills (rtl-spec-analysis, rustdv-testbench,
  rustdv-verify-cover).
- Repository work follows this file; session setup and close-out
  instructions are recorded here. Use the shipped examples and `/skills`.
- STATUS.md — authoritative implementation history and deviations log.
- CHANGELOG.md — user-facing, versioned release notes, including breaking
  changes and migration links. Use Unreleased during development; finalize the
  release date and tag comparison in the release commit before publication.
- README.md — project overview, book entry points, and repository map.

## Standing rules
- The pre-push hook, when installed, runs the full regression; keep it green.
  Verify claims by running things — transcripts in the book/README files are
  real output and must stay in sync with reruns.
- After editing Rust code, run `cargo fmt` and
  `cargo clippy --workspace --all-targets -- -D warnings` from the repository root before
  handing off the change.
- Read the relevant implementations or run the example before repeating a
  technical claim from documentation. STATUS notes and old handoffs are leads,
  not proof of current behavior.
- Preserve example source line counts when editing comments unless regenerating
  the affected transcripts: output embeds `file:line`. Keep manuscript and
  README transcripts aligned with the source after any such change.
- Report the platform actually tested. Linux results do not establish macOS/arm64
  behavior; record missing platform coverage in STATUS.md.

### Build storage and environment troubleshooting

- Put build products and scratch under `/tmp/rustdv-$(id -u)/`, with separate
  framework and example target directories. Shared fixed paths can belong to
  another user and become unwritable. The simulator launchers default to this
  per-user root; check overrides such as `CARGO_TARGET_DIR` and `SIM_BUILD_DIR`.
- Keep compiled simulator designs and loadable binaries out of the repository,
  including `.vpi`, `.vvp`, `.so`, `.dylib`, and object files. A synced Linux
  binary left in a Mac checkout can cause `vvp` to terminate with no output.
- In a synced or mounted checkout, avoid bulk deletion/recreation of directory
  trees; use temporary build trees and copy only intended source/doc changes.
- If a Git index operation cannot unlink `.git/index.lock`, resolve the actual
  filesystem permission restriction. Remove only a stale lock after confirming
  no Git process owns it. Do not rename or truncate locks as a workaround. Use
  `git --no-optional-locks status` for a read-only status check when needed.
- Check installed tools and current permissions rather than assuming an old
  offline VM's paths, network access, timeout, or toolchain drop still applies.
  Break long builds into resumable calls if the active environment requires it.
- A missing `lz4.h` or lz4 library prevents Verilator FST builds; check the
  dependencies in `sim/README.md` before attributing that failure to Rust code.
  An unavailable optional PDF renderer does not prevent an mdBook HTML build;
  follow `book-pdf/AGENTS.md` and report which output was verified.
- On `No space left on device`, inspect temporary build storage. Setting
  `CARGO_PROFILE_DEV_DEBUG=0` reduces debug artifacts; clean only the relevant
  temporary build/incremental directories owned by this task.

### Testing while iterating

Run from the repository root; select checks appropriate to the change. The
operating manual is `output/regression/TESTING.md`.

```sh
python3 output/regression/regress.py --suite unit
python3 output/regression/regress.py --suite book-sync
python3 output/regression/regress.py --filter ch37
python3 output/regression/regress.py --list
python3 output/regression/regress.py
```

Framework tests cover no-simulator unit behavior, targeted simulator behavior,
and compile-fail cases asserting the expected Rust error code. `ch21_macros`
is a macro demonstration with no simulator test: its `no_sim_test` entry in
`output/regression/regress.json` is intentional, not pending conversion.
Use `--list` for current test IDs and counts. Install the pre-push hook with
`python3 output/regression/regress.py --install-hook` when requested.

For a chapter simulator run, use the command in that chapter's README. The
general form, from `output/examples`, is:

```sh
sim-common/run_sim.sh <crate_name> <top_module> [hdl_files...]
```

Use `SIM=verilator` for the same launcher on Verilator. For framework unit
tests directly, run `cargo test --workspace` from the repository root with
`CARGO_TARGET_DIR` set under per-user temporary storage.

### Every version bump includes a feature and documentation audit

**Before handing off any framework version bump, verify what changed since
the last published release and update the documentation in the same session.**

1. Identify the last published version on crates.io and its source commit
   from the archive's `.cargo_vcs_info.json`. Confirm any matching Git tag
   points at that commit; a local version or tag alone does not prove what
   users received.
2. Compare public APIs, simulator scripts/host, and configuration controls
   against that baseline. Read the implementations and relevant tests;
   commit titles and STATUS notes are leads, not proof of a feature. Separate
   new features, fixes, tooling changes, and breaking API changes.
3. **Update CHANGELOG.md for every release/version bump.** Summarize added
   features, fixes, user-visible changes, and breaking changes from the
   verified published-source comparison. Include migration/build requirements
   and distinguish crate APIs from companion repository tools. Use Unreleased
   while developing the next version. When preparing the release commit, replace
   it with the intended release date and point the comparison at the planned
   release tag. Include these changes in the source commit that will be
   published and tagged, avoiding a follow-up bookkeeping commit. A dated
   changelog entry alone does not confirm publication on crates.io.
   Preserve historical entries and identify reconstructed history rather than
   guessing release dates.
   Update `README.md`, `rustdv/README.md`, `sim/README.md`, and affected verification
   skills/book passages where appropriate. Check option tables, runnable
   commands, migration advice, simulator/build requirements, and links.
   State when a feature also requires the repository's scripts or simulator
   host. Keep README.md's capability summary accurate and record the audit in
   a dated STATUS.md entry.
4. **Audit the book on every release, even when its existing examples pass.**
   Compare `rustdv/src/rustdv.rs`'s actual prelude and facade exports with the
   Toolkit and Appendix D, including which names require explicit imports.
   Review Chapter 17's signal/scheduler explanation, Chapter 40's launch,
   inspection, waveform, and coverage guidance, and Appendices B/C's
   translations against the implementation. Check affected chapters elsewhere
   too; distinguish missing feature coverage from incorrect existing claims.
   Update affected chapter README teaching notes and local instructions linked
   from the book guide and record the audit in STATUS.md. A green listing or
   transcript check proves agreement for examples already present, not that
   the prose or API catalogue is complete. Read `book-pdf/AGENTS.md`, preserve
   the teaching sequence and figure numbering, and keep optional advanced
   services beside their prerequisites. New book listings must have runnable
   source examples; do not bypass the listing checker with exemptions.
   Rebuild the book PDF, visually review the affected pages, and replace
   `Rust4RTLVerif.pdf` before the release commit. Follow the fresh-build and
   verification steps in `book-pdf/README.md`; an HTML-only build or skipped
   optional PDF backend does not complete this release step.
5. Keep release package versions, internal dependency requirements, and all
   affected lockfiles aligned. Compile-check new Rust examples and run the
   book-listing, transcript, and no-stale-version checks plus affected
   simulator regressions. Report environmental limitations precisely.
6. Keep reader-facing guides version-independent: use the crates.io badge,
   `cargo add`, and `rust-toolchain.toml`; put version-specific release history
   in CHANGELOG.md and dated implementation/validation history in STATUS.md.
   Check the packaged README and guide files with
   `cargo package --list` before publishing. A version bump does not authorize
   a commit, push, or publication, and a local bump is not a published release.

### Preparing the crates.io and GitHub release

Use the same source commit for the published crates and the GitHub release
tag. Follow the user's existing authorization; preparing notes or bumping
versions does not itself authorize committing, pushing, or publishing.

1. Finish the feature/book/changelog audit above, including rebuilding and
   reviewing the PDF and replacing `Rust4RTLVerif.pdf`. Include the refreshed
   PDF in the same release commit as the manuscript updates. Before that commit,
   replace Unreleased with the intended release date and finalize the comparison
   link at that version's tag. Commit these changes together with the intended
   release changes, following the user's authorization. Merge and use a clean
   checkout with passing release checks. Record its full commit SHA and the
   framework version from `rustdv/Cargo.toml`.
2. Run `cargo publish --workspace --exclude tinyalu_tb --exclude framework_tests
   --dry-run` from the repository root, building under per-user temporary
   storage. When publication is authorized, run the same command without
   `--dry-run`. Verify that all eight intended crate versions are available
   on crates.io before announcing success; a partially published workspace
   is not a completed release.
3. When tag creation/push is authorized, create the annotated tag `v` followed
   by that version at the recorded publication commit and push that specific
   tag to `origin`. Never let GitHub pick a newer default-branch commit and
   never move an existing published tag to a later documentation commit.
4. **Generate release notes directly from the changelog.**
   From the repository root, run `python3 scripts/release-notes.py VERSION`,
   substituting the framework version recorded in step 1. A leading `v` is
   accepted. The script extracts only that release's body from CHANGELOG.md,
   resolves relative documentation links at its GitHub tag, and changes the
   prospective comparison's `HEAD` to that tag. It rejects missing, duplicate,
   or empty entries before printing. The script writes only the notes to
   stdout; errors go to stderr with a nonzero exit status. Review the output
   and invoke it directly when creating the release; do not create a temporary
   notes file. The changelog remains the source of truth.
5. Hand the user an immediately usable command with the actual tag, title,
   and repository, using `gh release create`, `--verify-tag`, and notes captured
   directly from the script. With `version` set to the version from step 1:

   ```sh
   release_notes=$(python3 scripts/release-notes.py "$version") &&
     gh release create "v$version" --repo rustdv/rustdv --verify-tag \
       --title "rustdv $version" --notes "$release_notes"
   ```

   The `&&` stops release creation if extraction fails. State any remaining crate
   publication/tag prerequisites. If the user authorized creating the GitHub
   release, run that command after those prerequisites pass. A GitHub release
   does not publish crates, and publishing crates does not create a GitHub
   release in this repository's current setup.
6. Verify that the GitHub release tag identifies the actual crate-source commit
   and that all intended crate versions are published. The changelog date and
   comparison link are already finalized in step 1; no follow-up changelog
   commit is normally needed. Never move the release tag to later changes.
7. Check the separately maintained [rustdv website repository](https://github.com/rustdv/rustdv-website).
   Publishing crates or merging framework changes does not update rustdv.org.
   Copy the release's generated mdBook HTML into its `book/` directory,
   regenerate `introduction.html` and `translations.html` from the updated
   appendices, and refresh `getting-started.html`. Submit those changes as a
   separate website PR. Its deployment workflow publishes the site after merge;
   verify the live book, reference tables, installation guide, and links.
   docs.rs builds automatically after crates.io publication; verify the new
   crate versions' documentation builds succeeded.

### Leave no documentation debt — the rule that costs the most when ignored

**When you change code, update the artifacts that quote it, in the same
session.** This project's expensive failures have all been the same shape:
something changed, the prose describing it did not, and nothing noticed for
weeks. ch23/ch24/ch26 carried wrong `file:line` references and an undocumented
test; ch18–20's transcripts were 5ns stale after the DUT began self-clocking;
ch37's README described a chapter that does not exist. Each was cheap to fix at
the time and expensive to find later — one session spent ~20% of its context
paying that debt down.

**The same rule applies to this file and README.md.** A stale orientation
document costs more than stale prose, because every new thread reads it and
acts on it. If you finish work these files describe, update them in the same
session.

So, if you change:

| this | then regenerate |
|---|---|
| an example crate's code | its `README.md` figure map **and** its transcript |
| anything a transcript quotes (timing, paths, test count) | every README quoting it — `bash output/regression/verify-transcripts.sh` finds them |
| a `.rs` figure caption | the README figure map — they must agree, and caption edits stay line-count-neutral |
| framework syntax (an attribute, a name) | every call site, `skills/`, relevant API guides, and affected chapter README teaching notes and local instructions linked from `book-pdf/AGENTS.md` |
| `tinyalu_tb`'s code | the repository `README.md`, which walks it — both checks below cover it, so this one is enforced rather than remembered |

**Prefer a check to a note.** A rule written down is a rule someone must
remember; a rule in the regression is enforced. These checks exist because the
drift they catch went unnoticed for months while a full green regression ran
over it every push:

| check | what it gates |
|---|---|
| `custom/readme-transcripts` | every transcript in an example README, **in `book-pdf/src`, and in the repository `README.md`** is what the simulator prints (22 chapters) |
| `custom/book-listings` | every Rust listing in ch15–40 **and in the repository `README.md`** (ids `chRM/N`) is real code from that chapter's crate |
| `custom/book-figure-output` | every output block the manuscript prints for a goldened figure is what the program prints (ch35's standalone bins today) |
| `custom/no-stale-versions` | no reader-facing document hardcodes a rustdv version — the badge, `cargo add`, and `rust-toolchain.toml` carry those, and they update themselves |
| `book-sync` (pre-existing) | ch1–14 listings, byte-for-byte |

Run them directly while working — they are far cheaper than the full
regression: `python3 output/regression/verify-book-listings.py`,
`python3 output/regression/verify-no-stale-versions.py` (instant, no toolchain)
and `bash output/regression/verify-transcripts.sh`.

`no-stale-versions` encodes Ray's rule that **a version written into a file is
a staleness source**: it is right the day it is typed and wrong at the next
release, and nothing about releasing reminds anyone to go and fix it. Reach for
the mechanism that updates itself — the crates.io badge for the version,
`cargo add rustdv` for a dependency line, `rust-toolchain.toml` for the
compiler. Its `ALLOW` list is empty and is not a debt register; CHANGELOG.md,
STATUS.md, and the design log are out of scope because versioned release notes
and dated records are supposed to name the versions they describe.

`book-listings` separates two things that look alike and are not.
`QUOTED` holds listings that were never ours — the `Future` trait from the
standard library, a deliberately tidied macro expansion. Those are quotations,
carry no debt, and will not shrink. `KNOWN_DRIFT` is the debt register: a book
and a codebase disagreeing. **It is currently empty, and adding to it to make a
build pass is how drift comes back.** Keep the two apart; listing permanent
quotations as outstanding work makes a clean register look dirty and trains
everyone to ignore it.

If you find a class of error nothing catches, add the check rather than only
documenting the instance — and **make the check fail once on purpose before
trusting it.** Both of the above were mutation-tested that way; the first
version of `verify-transcripts.sh` reported success on chapters whose sims had
not run at all.

**Ask the process, not the prose.** A check that decides pass/fail by
pattern-matching human-readable output can be defeated by formatting. The
compile-fail suite tested "did it compile?" with `grep -E '^error'`; GitHub's
cargo emits ANSI colour, so the line began with an escape sequence, `^error`
never matched, and CI reported "compiled — no longer rejected" while printing
`error[E0277]` underneath. Exit codes, structured output and
`CARGO_TERM_COLOR=never` are not decoration — they are the difference between a
check and a guess.

**Do not create per-thread prompt files.** Repository instructions live here,
reader orientation in README.md, and history in STATUS.md. Point a new thread
at those files rather than creating another restatement that will go stale.

### Session close-out

Before ending repository work, inspect the final diff, run checks appropriate
to the change, and report any failures. Keep this file and README.md accurate,
remove advice the work made false, and keep branch names out of documents.
Record durable findings and validation in STATUS.md. Leave the tree in the
state the user requested: do not commit or push without authorization, and
do not create per-thread prompt files. These checks replace the removed
`close-out-thread` helper.

- Flag uncertainty openly (Open Questions / STATUS deviations) rather than
  presenting guesses as settled.
