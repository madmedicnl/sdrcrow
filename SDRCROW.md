# SDRCrow — the detached build

A copy of **SDR Oxide Brown** with a name, a config directory and a binary of
its own, so it can grow away from `dividebysandwich/sdroxide` without dragging
the fork's identity along. This file is the record of what was changed to get
here and what is still Brown underneath.

## What is already its own

| Thing | Now |
| --- | --- |
| The program's name | `SDRCrow` (`sdrcrow_version::FLAVOR`) |
| `--version` | `sdrcrow SDRCrow 2.3.0` |
| The binary and the crate | `sdrcrow` (`cargo build -p sdrcrow`, `target/release/sdrcrow`) |
| The config directory | `~/.config/sdrcrow` on Linux, `org.sdrcrow.sdrcrow` on macOS |
| The environment override | `SDRCROW_CONFIG_DIR` (`SDROXIDE_CONFIG_DIR` still honoured, for the tests) |
| The window's app id | `sdrcrow` (Wayland/WM class, `StartupWMClass` to match) |
| The release suffix | none — tags are `vX.Y.Z` |
| The Brown migration | removed: there is no upstream directory to copy from |
| The WSJT-X id, the "heard with" line | `SDRCrow` |
| Git remotes | **none** — this repository is detached from both `origin` and `upstream` |

## What is still called sdroxide

- Every crate (`sdrcrow-types`, `sdrcrow-radio`, `sdrcrow-ui`, …) and the
  imports between them. Mechanical, large, and safe to do on its own.
- `SDRCROW_VERSION`, the environment variable the build script sets.
- `packaging/` (the `.desktop` file, the icons), `.github/workflows/`,
  and the update check's feed, which still points at the Brown repository.
- The i18n domain (`sdroxide`) and the translations' ids.
- The engine's internal log targets (`sdrcrow_radio::…`).

## Build and run

```sh
cargo build --release -p sdrcrow
./target/release/sdrcrow            # its own config dir, created on first run
```

The first start makes `~/.config/sdrcrow` and starts from defaults: no
stations, no logbook, no themes. Copying a Brown installation over is a plain
`cp -a ~/.config/sdroxide-brown/. ~/.config/sdrcrow/` and is a decision, not a
default — this build never reads the other directory by itself.
