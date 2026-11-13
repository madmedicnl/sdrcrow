# SDRCrow

Software-defined-radio receiver and transceiver, detached from **SDR Oxide
Brown** and from the upstream project it forked. This file records what has
been changed to get there and what is still Brown underneath.

Repository: private, `madmedicnl/sdrcrow`. No upstream remote: nothing here
follows anyone else's history.

## Build and run

```sh
cargo build --release -p sdrcrow
./target/release/sdrcrow        # own config directory, created on first run
```

Installed on this machine: `~/.local/bin/sdrcrow`, a `.desktop` entry with
`Name=SDRCrow` and `StartupWMClass=sdrcrow`, and the emblem at every hicolor
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
| The environment override | `SDRCROW_CONFIG_DIR`; `SDROXIDE_CONFIG_DIR` still honoured, because the test harness sets it |
| The Wayland/WM app id | `sdrcrow` |
| The release suffix | none — tags are `vX.Y.Z` |
| The splash | the emblem, 3 s, dismissed at once by a click or a key |
| Packaging | `packaging/linux/sdrcrow.desktop`, `packaging/icons/sdrcrow-*.png` (from the emblem), udev rules `60-sdrcrow-*.rules` |
| The update check | asks `madmedicnl/sdrcrow` for releases, and nothing else |
| The i18n domain and catalogues | `sdrcrow`, `i18n/*/sdrcrow.ftl` |
| The web client | its page, manifest and title say SDRCrow |
| The Windows resource | `resource.rc` carries SDRCrow |
| Git remotes | **none** in the copy; `origin` is the private repository |

The Brown migration (copying upstream's shared config directory once) is gone:
there is no shared directory any more.

## What is still Brown

- **`SDROXIDE_CONFIG_DIR`** as a second accepted variable (test harness).
- **The documentation**: `README.md`, `AGENTS.md`, `TODO.md`, `CHANGELOG.md`,
  `docs/` and `BUILD-WINDOWS.md` are Brown's text and history. The house rules
  in `AGENTS.md` still describe how this work is done, and are worth keeping
  until they are rewritten with the new name.
- **The rest of the packaging**: `packaging/windows`, `packaging/macos`,
  `packaging/snap`, the AppImage and the `.deb` container scripts still carry
  the Brown names and logos.
- **The engine's shared resources**: SDRCrow and Brown both take the first
  RSP/dongle, TCI port 50001, the server port and rigctld's. Two of them at
  once fight; running both needs at least one of those made configurable per
  program.
- **The CI**: `.github/workflows` was removed with the detach (it was Brown's
  release, nightly and packaging machinery, publishing under this repository's
  name and secrets). There is none yet.
