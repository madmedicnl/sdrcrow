#!/usr/bin/env python3
"""Bring SDR Oxide Brown's new commits over into SDRCrow, renamed.

SDRCrow was renamed from Brown in one pass: 67 crate directories, every
`use sdroxide_*`, the build variable, the config directory, the program name.
Brown keeps moving — it is where the work happens — so its commits are ported
here, one by one, through the same rename, rather than merged: a merge would
conflict in every file the rename touched.

    tools/sync-from-brown.py            # port everything new, then stop
    tools/sync-from-brown.py --dry-run  # say what would be ported
    tools/sync-from-brown.py --base SHA # pretend the last ported commit is SHA

The last ported commit is kept in `refs/brown-synced`, a git ref rather than a
file, so it travels with the repository and cannot be forgotten in a merge.

Nothing is pushed and nothing is committed beyond the ported commits
themselves: the run leaves the tree for a person to test and push, which is the
point of doing it daily rather than automatically.
"""

from __future__ import annotations

import re
import subprocess
import sys

REMOTE = "brown"
URL = "https://github.com/madmedicnl/sdroxide-brown.git"
MARKER = "refs/brown-synced"

# The rename, in order. Anything that must survive untouched is held out first:
# `SDRCROW_CONFIG_DIR` is this program's own variable now, and the vendored
# crates' provenance notes are third-party text.
PROTECT = {"SDRCROW_CONFIG_DIR": "\x00CFG\x00"}
RENAMES = [
    ("sdroxide-brown", "sdrcrow"),
    ("sdroxide-", "sdrcrow-"),
    ("sdroxide_", "sdrcrow_"),
    ("SDROXIDE_VERSION", "SDRCROW_VERSION"),
    ("SDR Oxide Brown", "SDRCrow"),
    ("SDR Oxide", "SDRCrow"),
]

# Brown's files SDRCrow does not keep. A commit that touches one of these is
# ported without it, and the skip is printed.
DROP_PREFIXES = (".github/",)


def git(*args: str) -> str:
    res = subprocess.run(["git", *args], capture_output=True, text=True)
    if res.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {res.stderr.strip()}")
    return res.stdout


def rename_patch(text: str) -> str:
    for k, v in PROTECT.items():
        text = text.replace(k, v)
    for old, new in RENAMES:
        text = text.replace(old, new)
    for k, v in PROTECT.items():
        text = text.replace(v, k)
    return text


def drop_ignored_files(patch: str) -> tuple[str, list[str]]:
    """Remove the per-file sections Brown keeps and SDRCrow does not."""
    out: list[str] = []
    dropped: list[str] = []
    keep = True
    for line in patch.splitlines(keepends=True):
        if line.startswith("diff --git "):
            path = line.split(" b/", 1)[-1].strip()
            keep = not path.startswith(DROP_PREFIXES)
            if not keep:
                dropped.append(path)
        if keep:
            out.append(line)
    return "".join(out), dropped


def main(argv: list[str]) -> int:
    dry = "--dry-run" in argv
    base = None
    if "--base" in argv:
        base = argv[argv.index("--base") + 1]

    if REMOTE not in git("remote").split():
        if not dry:
            git("remote", "add", REMOTE, URL)
        print(f"+ remote {REMOTE} -> {URL}")

    git("fetch", "--quiet", REMOTE, "main")
    tip = git("rev-parse", f"{REMOTE}/main").strip()

    if base is None:
        try:
            base = git("rev-parse", "--verify", "--quiet", MARKER).strip()
        except SystemExit:
            base = git("merge-base", "HEAD", f"{REMOTE}/main").strip()
            print(f"! no {MARKER}: starting from the merge base {base[:10]}")

    commits = git("rev-list", "--reverse", "--no-merges", f"{base}..{tip}").split()
    if not commits:
        print(f"nothing new: {REMOTE}/main is at {tip[:10]}, already ported")
        return 0

    print(f"{len(commits)} commit(s) to port, {base[:10]}..{tip[:10]}")
    for sha in commits:
        subject = git("log", "-1", "--format=%s", sha).strip()
        print(f"\n== {sha[:10]} {subject}")
        if dry:
            continue
        patch = git("format-patch", "-1", "--stdout", "--binary", sha)
        patch, dropped = drop_ignored_files(rename_patch(patch))
        for path in dropped:
            print(f"   skipped (not in this repository): {path}")
        res = subprocess.run(
            ["git", "am", "--3way", "-q"], input=patch, capture_output=True, text=True
        )
        if res.returncode != 0:
            print(res.stderr.strip()[:2000])
            print(
                "\nstopped here: resolve the conflict, `git am --continue`, then run this "
                "script again — it picks up from the commit after this one."
            )
            return 1
    if not dry:
        git("update-ref", MARKER, tip)
        print(f"\nported up to {tip[:10]}; {MARKER} moved")
        print("Now: cargo check --workspace --all-targets, the tests of what moved, "
              "tools/check-names.sh, then push.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
