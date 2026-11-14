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

The last ported commit is kept in `refs/brown-synced` and in `.brown-synced`:
the ref is the fast answer, the file is the one a fresh clone has.

Nothing is pushed and nothing is committed beyond the ported commits
themselves: the run leaves the tree for a person to test and push, which is the
point of doing it daily rather than automatically.
"""

from __future__ import annotations

import pathlib
import re
import subprocess
import sys

REMOTE = "brown"
URL = "https://github.com/madmedicnl/sdroxide-brown.git"
MARKER = "refs/brown-synced"
# The same fact in a file, because a git ref lives in one clone. A fresh clone
# has no ref, and the merge base with Brown is then the commit this program was
# cut from — which would make the tool offer to port months of commits it
# already has. At the root rather than under `docs/internal/`, which is
# git-ignored: a marker nobody commits is a marker a fresh clone does not have.
MARKER_FILE = ".brown-synced"
# Where the repository is, so `patch` and the commit run in the right tree even
# when the script is called from somewhere else.
REPO = subprocess.run(
    ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
).stdout.strip() or "."

# The rename, in order: the longer spellings first, so `sdroxide-brown` becomes
# `sdrcrow` rather than `sdrcrow-brown`, and `SDROXIDE_VERSION` becomes
# `SDRCROW_VERSION` before the catch-all sees it. Anything held out in PROTECT
# is restored afterwards untouched.
PROTECT = {
    # Citations: a comment that points at an upstream issue, or at Brown's own
    # manual, is telling the reader where something came from. Renaming those
    # would break the link and lose the provenance.
    "github.com/dividebysandwich/sdroxide": "\x00UP\x00",
    "github.com/madmedicnl/sdroxide-brown": "\x00BROWN\x00",
}
RENAMES = [
    ("sdroxide-brown", "sdrcrow"),
    ("sdroxide-", "sdrcrow-"),
    ("sdroxide_", "sdrcrow_"),
    ("SDR Oxide Brown", "SDRCrow"),
    ("SDR Oxide", "SDRCrow"),
    ("SDRoxide", "SDRCrow"),
    ("SDROXIDE_", "SDRCROW_"),
    ("SDROXIDE", "SDRCROW"),
    ("Sdroxide", "Sdrcrow"),
    ("sdroxide", "sdrcrow"),
]

# Brown's files SDRCrow does not keep. A commit that touches one of these is
# ported without it, and the skip is printed.
DROP_PREFIXES = (".github/",)


def git(*args: str) -> str:
    res = subprocess.run(["git", *args], capture_output=True, text=True)
    if res.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {res.stderr.strip()}")
    return res.stdout


def git_bytes(*args: str) -> bytes:
    """A patch, which is bytes: the tree holds files that are not UTF-8.

    An old commit with a Latin-1 byte in a comment would stop the port at the
    decode otherwise. `surrogateescape` carries those bytes through untouched.
    """
    res = subprocess.run(["git", *args], capture_output=True)
    if res.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {res.stderr.decode('utf-8', 'replace').strip()}")
    return res.stdout


def rename_patch(data: bytes) -> bytes:
    text = data.decode("utf-8", "surrogateescape")
    for k, v in PROTECT.items():
        text = text.replace(k, v)
    for old, new in RENAMES:
        text = text.replace(old, new)
    for k, v in PROTECT.items():
        text = text.replace(v, k)
    return text.encode("utf-8", "surrogateescape")


def drop_ignored_files(patch: bytes) -> tuple[bytes, list[str]]:
    """Remove the per-file sections Brown keeps and SDRCrow does not."""
    out: list[bytes] = []
    dropped: list[str] = []
    keep = True
    for line in patch.splitlines(keepends=True):
        if line.startswith(b"diff --git "):
            path = line.decode("utf-8", "replace").split(" b/", 1)[-1].strip()
            keep = not path.startswith(DROP_PREFIXES)
            if not keep:
                dropped.append(path)
        if keep:
            out.append(line)
    return b"".join(out), dropped


def apply_commit(sha: str) -> bool:
    """Put one Brown commit on this branch, renamed. False if a person must look.

    `git am` is tried first: it keeps the message, the author and the date, and
    it is the right tool when the context still matches. It stops mattering as
    soon as this repository has been edited where Brown also edited — the blob
    hashes in the patch are Brown's, so `--3way` has no base to merge against and
    the patch is applied literally, which fails on the first moved line.

    Then `patch --merge` takes over, which merges against *this* file rather than
    the blobs the patch was cut from. It leaves conflict markers in the files it
    could not settle, and prints `NOT MERGED` for a hunk it dropped on the floor;
    either stops the port right there, with the tree for a person to finish.
    """
    patch, dropped = drop_ignored_files(rename_patch(
        git_bytes("format-patch", "-1", "--stdout", "--binary", sha)))
    for path in dropped:
        print(f"   skipped (not in this repository): {path}")

    res = subprocess.run(["git", "am", "--3way", "-q"], input=patch, capture_output=True)
    if res.returncode == 0:
        return True

    # `git am` refuses a mailbox patch it cannot apply at all and leaves the
    # tree clean; if it did apply something, put it back before trying again.
    if subprocess.run(["git", "am", "--abort"], capture_output=True).returncode == 0:
        print("   git am could not place it; merging into these files instead")

    for line in patch.decode("utf-8", "replace").splitlines():
        if line.startswith("diff --git "):
            path = line.split(" b/", 1)[-1].strip()
            subprocess.run(["mkdir", "-p", str(pathlib.Path(path).parent)])

    res = subprocess.run(
        ["patch", "-p1", "--merge", "--no-backup-if-mismatch"],
        input=patch, capture_output=True, cwd=REPO,
    )
    out = res.stdout.decode("utf-8", "replace")
    troubled = [ln.split(":")[-1].strip() for ln in out.splitlines() if "NOT MERGED" in ln]
    if res.returncode != 0 or troubled:
        print(out.strip()[:1500])
        print("\nstopped here: `patch` could not place these hunks —")
        for path in troubled:
            print(f"   {path}")
        print("finish the commit by hand, then run this script again.")
        return False

    marked = subprocess.run(
        ["git", "grep", "-l", "-E", "^<<<<<<< $|^>>>>>>> $", "--", "."],
        capture_output=True, text=True,
    ).stdout.split()
    if marked:
        print("\nstopped here: these files hold conflict markers —", ", ".join(marked))
        print("resolve them, then run this script again.")
        return False

    author = git("log", "-1", "--format=%an <%ae>", sha).strip()
    date = git("log", "-1", "--format=%aI", sha).strip()
    message = git("log", "-1", "--format=%B", sha)
    git("add", "-A")
    res = subprocess.run(
        ["git", "commit", "-q", f"--author={author}", f"--date={date}", "-F", "-"],
        input=message, capture_output=True, text=True,
    )
    if res.returncode != 0:
        print(res.stderr.strip()[:1000])
        return False
    print(f"   committed {git('rev-parse', '--short', 'HEAD').strip()} "
          f"(author {author})")
    return True


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
            recorded = pathlib.Path(REPO, MARKER_FILE)
            if recorded.is_file():
                base = recorded.read_text(encoding="utf-8").split()[0]
                print(f"! no {MARKER}: {MARKER_FILE} says {base[:10]}")
            else:
                base = git("merge-base", "HEAD", f"{REMOTE}/main").strip()
                print(f"! no {MARKER} and no {MARKER_FILE}: "
                      f"starting from the merge base {base[:10]}")

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
        if not apply_commit(sha):
            return 1
    if not dry:
        git("update-ref", MARKER, tip)
        recorded = pathlib.Path(REPO, MARKER_FILE)
        recorded.parent.mkdir(parents=True, exist_ok=True)
        recorded.write_text(tip + "\n", encoding="utf-8")
        git("add", MARKER_FILE)
        print(f"\nported up to {tip[:10]}; {MARKER} and {MARKER_FILE} moved")
        # The rename is only as good as this: something Brown wrote that the
        # list above does not know about shows up here, before it is pushed.
        check = subprocess.run(["tools/check-names.sh"], capture_output=True, text=True)
        print(check.stdout.strip() or check.stderr.strip())
        if check.returncode != 0:
            print(
                "\nthe old name came through the port: teach `RENAMES` the spelling it "
                "used, then run this script again for the same commit."
            )
            return 1
        print("Now: cargo check --workspace --all-targets, the tests of what moved, "
              "then push.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
