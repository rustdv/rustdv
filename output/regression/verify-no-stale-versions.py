#!/usr/bin/env python3
"""Check that no reader-facing document hardcodes a rustdv version number.

    python3 output/regression/verify-no-stale-versions.py [--list]

Ray's rule, and the reason this exists: **a version written into a file is a
staleness source.** It is correct on the day it is typed and wrong on the day
of the next release, and nothing about the release reminds anyone to go and
change it. The repository had exactly that — its packaged crates.io README
told readers for weeks after the 0.1 launch
that crates.io held "the 0.0.1 name reservation" and that `cargo add rustdv`
would work "once rustdv 0.1 ships".

The mechanisms that do not go stale, and what each replaces:

    a version in prose        -> the crates.io badge, which renders live
    `rustdv = "0.1"` in TOML  -> `cargo add rustdv`, which writes it for you
    an MSRV sentence          -> `rust-toolchain.toml`, which rustup obeys

Scope is current reader guides. **CHANGELOG.md, STATUS.md, and the design log
are exempt on purpose**: they are versioned or dated historical records, and
a record of what ran on which version in July is supposed to still say July.
Generated trees
(`rustdv/target/`, `book-pdf/book/`) are build output and are not edited by
hand.

This checks prose, not manifests. Real `Cargo.toml` files must pin versions —
that is what a manifest is for.
"""
import re, sys, glob, os

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
os.chdir(ROOT)

# Reader-facing markdown: what someone outside this project reads.
SCOPE = [
    "README.md",
    "CONTRIBUTING.md",
    "rustdv/README.md",
    "rustdv/tinyalu_tb/README.md",
    "sim/README.md",
    "book-pdf/src/*.md",
    "output/examples/README.md",
    "output/examples/*/README.md",
    "skills/*.md",
    "skills/*/*.md",
]

# Two shapes of the same mistake.
PATTERNS = [
    # `rustdv = "0.1"` / `rustdv = { version = "0.1.1", ... }` — a dependency
    # line a reader would copy, which pins them to whatever was current when
    # the sentence was written.
    (re.compile(r'\brustdv[a-z-]*\s*=\s*(?:\{[^}\n]*version\s*=\s*)?"[0-9]'),
     'a pinned rustdv version — say `cargo add rustdv` instead'),
    # "rustdv 0.1.1", "rustdv v0.1" — a version named in running prose.
    (re.compile(r'\brustdv\s+v?[0-9]+\.[0-9]+(\.[0-9]+)?\b'),
     'a rustdv version in prose — let the crates.io badge carry it'),
]

# A line may be exempt where naming a version is the document's actual subject
# rather than an oversight. **This is currently empty, and it is not a register
# of things to fix later** — if a version belongs in reader-facing prose, the
# entry says why and stays. Adding one to make a build pass is how the staleness
# comes back.
ALLOW = {
    # (path, substring that must appear in the offending line): why
}


def allowed(path, line):
    for (p, needle), _ in ALLOW.items():
        if path == p and needle in line:
            return True
    return False


def files():
    seen = []
    for pat in SCOPE:
        for f in sorted(glob.glob(pat)):
            if f not in seen and os.path.isfile(f):
                seen.append(f)
    return seen


def main():
    scanned = 0
    hits = []
    for path in files():
        scanned += 1
        for n, line in enumerate(open(path, encoding="utf-8"), 1):
            for rx, why in PATTERNS:
                if rx.search(line) and not allowed(path, line):
                    hits.append((path, n, line.rstrip(), why))

    print(f"reader-facing docs scanned for hardcoded rustdv versions: "
          f"{scanned} files, {len(hits)} hit(s)")
    if hits:
        print("  A version in prose goes stale at the next release and nothing")
        print("  reminds anyone to update it. Use the live mechanism instead:")
        print("    crates.io badge (version) / `cargo add rustdv` (dependency)")
        print("    / rust-toolchain.toml (compiler).")
        for path, n, line, why in hits:
            print(f"    {path}:{n}: {why}")
            print(f"        {line.strip()}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
