#!/usr/bin/env python3
"""Print a version's CHANGELOG.md entry to stdout for a GitHub release."""

import argparse
import re
from pathlib import Path
from typing import Optional
from urllib.parse import urljoin, urlsplit

ROOT = Path(__file__).resolve().parents[1]
GITHUB = "https://github.com/rustdv/rustdv"


def extract_notes(changelog: str, version: str) -> str:
    """Select one exact level-two release heading, excluding fenced examples."""
    entries: list[list[str]] = []
    current: Optional[list[str]] = None
    fence: Optional[str] = None
    for line in changelog.splitlines(keepends=True):
        marker = re.match(r"^ {0,3}(`{3,}|~{3,})", line)
        if marker:
            token = marker[1]
            if fence is None:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = None
        heading = (
            re.match(r"^##[ \t]+(\S+)(?:[ \t].*)?$", line) if fence is None else None
        )
        if heading:
            current = [] if heading[1].removeprefix("v") == version else None
            if current is not None:
                entries.append(current)
        elif current is not None:
            current.append(line)
    if not entries:
        raise ValueError(f"no changelog entry for {version}")
    if len(entries) != 1:
        raise ValueError(f"multiple changelog entries for {version}")
    notes = "".join(entries[0]).strip()
    if not notes:
        raise ValueError(f"empty changelog entry for {version}")
    return notes + "\n"


def release_links(notes: str, version: str) -> str:
    """Resolve inline documentation links and the prospective comparison."""
    tag = f"v{version}"

    def replace(match: re.Match[str]) -> str:
        target = match[1]
        if not urlsplit(target).scheme and not target.startswith(("#", "//")):
            target = urljoin(f"{GITHUB}/blob/{tag}/", target)
        if target.startswith(f"{GITHUB}/compare/") and target.endswith("...HEAD"):
            target = target.removesuffix("HEAD") + tag
        return f"]({target})"

    return re.sub(r"\]\(([^\s)]+)\)", replace, notes)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", help="release version, with or without a leading v")
    args = parser.parse_args()
    version = args.version.removeprefix("v")
    if not re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version
    ):
        parser.error("version must look like 1.2.3 or v1.2.3")
    try:
        notes = extract_notes(
            (ROOT / "CHANGELOG.md").read_text(encoding="utf-8"), version
        )
        notes = release_links(notes, version)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(notes, end="")


if __name__ == "__main__":
    main()
