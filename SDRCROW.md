# SDRCrow

Software-defined-radio receiver and transceiver, detached from **SDR Oxide
Brown** and from the upstream project it forked. This file records what has
been changed to get there, how Brown's ongoing work is brought across, and
what is still Brown underneath.

Repository: private, `madmedicnl/sdrcrow`. One remote, `origin`. A second,
`brown`, is read-only and exists only so its commits can be ported by hand.

## Build and run

```sh
cargo build --release -p sdrcrow
./target/release/sdrcrow        # own config directory, created on first run
```

Installed on this machine: `~/.local/bin/sdrcrow`, a `.desktop` entry with
`Name=SDRCrow` and `StartupWMClass=sdrcrow`, and the logo at every hicolor
size — so it can be started from the launcher as well as from a shell.

First start makes `~/.config/sdrcrow` and begins from defaults: no stations,
no logbook, no themes. Copying a Brown installation over is a plain
`cp -a ~/.config/sdroxide-brown/. ~/.config/sdrcrow/` and is a decision, not
a default — SDRCrow never reads the other directory by itself.

## Bringing settings over

A first start offers the configurations it finds on the machine — the Brown
build's (`~/.config/sdroxide-brown`) and upstream's (`~/.config/sdroxide`), by
name, path and size — and copies the one chosen into `~/.config/sdrcrow`.

- **Copy, never move**: the other program is untouched and this directory is
  never overwritten.
- **Asked once**: the answer is `UiSettings::config_import_asked`, so a reset to
  defaults (which rewrites the settings) brings the question back.
- **Restart, not quit**: the copied files are read at start-up, so the dialog's
  button starts this program again with the arguments it had, a moment after
  the old one closes — the receiver, the sound card and the TCI port are let go
  before the new instance asks for them.

## Keeping up with Brown

Brown is where the work happens, so its commits are brought here daily. Not by
merging — the rename touched 1155 files, and a merge would conflict in every
one Brown has also changed. They are *ported* instead, one commit at a time,
through the rename:

```sh
tools/sync-from-brown.py --dry-run   # say what is new, change nothing
tools/sync-from-brown.py             # port it, then stop
cargo check --workspace --all-targets
tools/check-names.sh                 # the old name has not crept back in
```

- `refs/brown-synced` holds the last ported Brown commit, and
  `docs/internal/BROWN-SYNCED` says the same thing in a file: a ref lives in one
  clone, and without the file a fresh clone would take the merge base — the
  commit this program was cut from — and offer months of commits it already
  has.
- Each commit keeps its message, author and date. `git am` is tried first; when
  our edits have moved the context it hands over to `patch --merge`, which
  merges against the files here instead of the blobs the patch was cut from.
  A hunk neither can place, or conflict markers left in a file, stops the port
  on that commit with the tree for a person to finish — then run the tool again
  and it picks up the next one.
- The rename is the list at the top of the tool: `sdroxide-` → `sdrcrow-`,
  `sdroxide_` → `sdrcrow_`, `SDRoxide`/`SDR Oxide` → `SDRCrow`,
  `SDROXIDE_VERSION` → `SDRCROW_VERSION`, and `sdroxide-brown` → `sdrcrow` for
  the directories and app ids. When Brown adds a new spelling of its own name,
  that list is where it is taught.
- `.github/` is dropped on the way in: those files are Brown's release and
  packaging machinery and do not exist here.
- Nothing is pushed and nothing is merged: the run leaves the tree for a person
  to test, which is the point of it being daily rather than automatic.

`tools/check-names.sh` is the rule as a gate: the old name may survive in the
documentation's history, and in the two places that must spell it — the port
itself, and the first-start import, which names the other programs so the
operator recognises one.

## What is its own

| Thing | Now |
| --- | --- |
| The program's name | `SDRCrow` (`sdrcrow_version::FLAVOR`) |
| `--version` | `sdrcrow SDRCrow 1.0.0` |
| The binary and the crate | `sdrcrow` |
| Every crate | `sdrcrow-*` (67 of them), and every `use sdrcrow_*` with them |
| Log targets | `sdrcrow_radio::…`, `sdrcrow_digi::…` (they follow the crate names) |
| The build variable | `SDRCROW_VERSION` |
| The config directory | `~/.config/sdrcrow`, `org.sdrcrow.sdrcrow` on macOS |
| The environment override | `SDRCROW_CONFIG_DIR`, and only that name |
| The Wayland/WM app id | `sdrcrow` |
| The release suffix | none — tags are `vX.Y.Z` |
| The look | `UiTheme::Default` is SDRCrow's own: charcoal on a near-black page, the logo's gold on the chrome, its blue as the accent, steel ink and red kept for alerts — and it is the fallback every unknown value lands on. Rainbow keeps Brown's old navy instrument |
| The manual's pictures | `tools/manual_shot.py` takes a settings page from the running program on a throw-away configuration; `docs/internal/SCREENSHOT-PLAN.md` says which pictures still need the operator |
| The splash | the SDRCrow logo (`crates/sdrcrow-ui/assets/splash.png`), 2 s, dismissed at once by a click or a key |
| Packaging | `packaging/linux/sdrcrow.desktop`, `packaging/icons/sdrcrow-*.png` (the logo, 16 to 512 px), udev rules `60-sdrcrow-*.rules`, `packaging/windows/sdrcrow.ico` and a fresh UpgradeCode, `packaging/macos/sdrcrow.icns` and bundle id `org.sdrcrow.sdrcrow`, headless installers naming SDRCrow |
| The update check | asks `madmedicnl/sdrcrow` for releases, and nothing else |
| The i18n domain and catalogues | `sdrcrow`, `i18n/*/sdrcrow.ftl` |
| The web client | its page, manifest, JS bridge objects and title say SDRCrow |
| The Windows resource | `resource.rc` carries SDRCrow |

The Brown migration (copying upstream's shared config directory once) is gone:
there is no shared directory any more.

## What is still Brown

- **The documentation**: `README.md`, `AGENTS.md`, `TODO.md`, `CHANGELOG.md`,
  `docs/` and `BUILD-WINDOWS.md` are Brown's text and history, kept because the
  house rules in them still describe how this work is done. The manual's
  screenshots were taken in Brown's navy look, not in this program's. This file is the one
  that says what SDRCrow is.
- **The engine's shared resources**: SDRCrow and Brown both take the first
  RSP/dongle, TCI port 50001, the server port and rigctld's. Two of them at
  once fight; running both needs at least one of those made configurable per
  program.
- **The CI**: `.github/workflows` was removed with the detach (it was Brown's
  release, nightly and packaging machinery, publishing under this repository's
  name and secrets). There is none yet.
- **The Windows, macOS, snap and AppImage builds**: the names in them are
  SDRCrow's now, but none of them has been built since the detach. Linux is the
  only platform this program has actually been packaged and run on.
