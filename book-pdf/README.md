# Building *Rust for RTL Verification*

This directory is an [mdBook](https://rust-lang.github.io/mdBook/) project.
The chapter sources live in `src/` (ordered by `src/SUMMARY.md`); everything
under `book/` is generated output and is gitignored — never edit it by hand.

Editorial and agent instructions are in [AGENTS.md](AGENTS.md).

## Prerequisites

Both tools install through Cargo. Install Rust with rustup; this repository's
[`rust-toolchain.toml`](../rust-toolchain.toml) selects the compiler when you
run Cargo inside the clone:

```sh
cargo install mdbook --locked       # the book builder
cargo install mdbook-pdf --locked   # the PDF backend
```

`mdbook-pdf` produces the PDF by driving a headless Chrome/Chromium, so a
Chrome or Chromium installation must be present on the machine. Both tools
must be on `PATH`. If the active environment cannot install the PDF backend,
an HTML build still checks the book's structure. The PDF backend is optional
in `book.toml`, so a successful mdBook exit alone does not prove a PDF was
produced. Release preparation requires an actual PDF rebuild and review.

## Build

```sh
cd book-pdf
mdbook build
```

Because two backends are configured in `book.toml` (`[output.html]` and
`[output.pdf]`), the outputs land in per-backend subdirectories:

| Output | Location |
|---|---|
| Website | `book/html/index.html` |
| PDF | `book/pdf/output.pdf` |

## Refreshing the release PDF

Before the release commit, build into a fresh temporary directory from the
repository root so an old PDF cannot be mistaken for a successful rebuild:

```sh
mkdir -p "/tmp/rustdv-$(id -u)"
book_build=$(mktemp -d "/tmp/rustdv-$(id -u)/release-book.XXXXXX")
mdbook build book-pdf --dest-dir "$book_build" &&
  test -s "$book_build/pdf/output.pdf"
```

Open the generated PDF and review the changed chapters, tables, code blocks,
and page numbering. Confirm the new text appears and nothing is clipped or
overlapping. If the backend was skipped or failed, resolve that failure before
replacing the root copy. After review, from the same shell:

```sh
cp "$book_build/pdf/output.pdf" Rust4RTLVerif.pdf
```

Include `Rust4RTLVerif.pdf` in the same release commit as the manuscript and
final changelog. The file at the repository root is the distributed book;
the build directory is generated output. Never edit either PDF by hand.

## Live preview while writing

```sh
cd book-pdf
mdbook serve --open
```

This rebuilds on every save and refreshes the browser. `serve` runs only
the HTML backend — run `mdbook build` when you want a fresh PDF.

## Styling

- `theme/` — mdBook theme overrides (index.hbs, css, fonts, favicons).
- `github-markdown.css` — additional stylesheet listed in `book.toml`.

## Adding a chapter

1. Create `src/chapter-NN-short-name.md`.
2. Add it to `src/SUMMARY.md` in reading order — a chapter missing from
   SUMMARY.md is not built.
3. `mdbook build`.
