#!/bin/sh
# **The old name does not appear in this program.**
#
# SDRCrow descends from SDR Oxide Brown, which forked upstream, and the port of
# Brown's commits (tools/sync-from-brown.py) rewrites the name on the way in.
# This is the check that it worked: the name may survive in the documentation's
# history — what happened, when, and under what name — and nowhere else.
#
# Run from the repository root. Exits non-zero and prints every offending line.
set -u

# Two things are allowed besides the documentation's history: the two files
# that *are* the rename (this one and the port), and citations — a comment that
# points at an upstream issue or at Brown's own manual is telling the reader
# where something came from, not naming this program.
allowed_files="tools/sync-from-brown.py tools/check-names.sh crates/sdrcrow-config/src/lib.rs"

status=0
hits=$(
    git grep -n -I -i "sdroxide" -- \
        ':!docs/*' \
        ':!*.md' \
        ':!target/*' \
        ':!Cargo.lock' \
        2>/dev/null | grep -v -E "^($(echo "$allowed_files" | tr ' ' '|')):" \
        | grep -v -E "github\.com/(dividebysandwich/sdroxide|madmedicnl/sdroxide-brown)" || true
)

if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    printf '\n%s\n' "the old name is in the tree outside the documentation's history"
    status=1
fi

# The program's own name in a place that would be visible to an operator. The
# import is exempt for the reason above: it names the other programs.
hits=$(git grep -n -I "SDR Oxide" -- 'crates/*' 'src/*' 'packaging/*' 2>/dev/null \
    | grep -v -E "^(crates/sdrcrow-config/src/lib\.rs):" || true)
if [ -n "$hits" ]; then
    printf '%s\n' "$hits"
    printf '%s\n' "SDR Oxide is still a name this program gives itself"
    status=1
fi

# A conflict block in a tracked file is worse than either side of it: the port
# merges against this tree with `patch --merge`, which writes markers when it
# cannot settle a hunk, and a marker left behind compiles in Markdown and not in
# code. Root files included: the first scan of this kind missed AGENTS.md and
# TODO.md because it only looked under `crates/`, `docs/` and `tools/`.
markers=$(git grep -n -E "^(<<<<<<<|=======|>>>>>>>)$" -- . || true)
if [ -n "$markers" ]; then
    printf '%s\n' "$markers"
    printf '%s\n' "a conflict block was left in a tracked file"
    status=1
fi

if [ "$status" -eq 0 ]; then
    printf '%s\n' "ok: only the documentation keeps the old name, and no conflict block is left"
fi
exit "$status"
