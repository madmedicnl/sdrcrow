# SDRCrow

<center><img width="626" height="626" alt="SDRCROW" src="https://github.com/user-attachments/assets/b3e14ac0-f102-4846-bbc3-0bebae4f63e1" /></center>



A software-defined-radio receiver and transceiver: a native desktop application,
a browser client served from the same engine, CAT and network radios, digital
modes and decoding, and a logbook. This is a detached continuation of **SDR
Oxide Brown** — its own program, its own configuration, its own name.

```sh
cargo build --release -p sdrcrow
./target/release/sdrcrow
```

First start creates `~/.config/sdrcrow` and begins from defaults. The old
Brown installation lives beside it in `~/.config/sdroxide-brown` and is neither
read nor touched; copying it across is a `cp -a` and a decision.
`SDRCROW_CONFIG_DIR` points the program at another profile.

## Download

The Linux, macOS and Windows builds and the web client are on the
[latest release](https://github.com/madmedicnl/sdrcrow/releases):

| file (`<v>` is the version, e.g. `1.2.0`) | what |
|---|---|
| `sdrcrow-v<v>-linux-x86_64.tar.gz` | the standard Linux build — CAT, OpenHPSDR, TCI, RTL-SDR, Airspy, RX-888, ELAD; no SoapySDR needed |
| `sdrcrow-v<v>-linux-x86_64.deb` | the same as a Debian package |
| `sdrcrow-v<v>-linux-x86_64-soapysdr.deb` | with SoapySDR device support |
| `sdrcrow-v<v>-linux-x86_64.AppImage` | one file, run it |
| `sdrcrow-v<v>-macos-x86_64.dmg` / `.tar.gz` | macOS, Intel |
| `sdrcrow-v<v>-macos-aarch64.dmg` / `.tar.gz` | macOS, Apple silicon |
| `sdrcrow-v<v>-windows-x86_64.msi` / `.zip` | Windows installer and portable folder |
| `sdrcrow-web-v<v>.zip` | the browser client, to serve from a web server |
| `SHA256SUMS` | the checksums |

The macOS and Windows builds come from `.github/workflows/release.yml`; the
`-soapysdr` variants of each platform are built the same way. Everything below
builds from source.

## The ultra narrowband voice experiment (UNBV)

The **UNBV** chip under *Experimental* in the band menu opens an experiment after
**KC8COM's Ultra Narrowband Voice Experimenter**: recorded speech is slowed by a
factor (2 to 100) and shifted into a narrow slice of the sideband, and the other
station shifts it back and speeds it up. At factor 50 the voice sits in about
46 Hz around 1 kHz, and a phrase takes fifty times as long to send. It rides
USB/LSB — LSB on 160/80/40 m, USB elsewhere, and the window lets you overrule —
with the identification in Morse between the blocks. It is an experiment, not a
mode; section 18 of the [manual](docs/USER_MANUAL.md) has the details.

## Where things are

- **`SDRCROW.md`** — what was changed to detach this build, and what is still
  Brown underneath (the i18n domain, the old config variable, the packaging for
  other platforms).
- **`tools/build-quickstart-pdfs.py`** — rebuilds the quickstart PDFs from their
  Markdown, with the emblem on the cover and a contents list; it needs
  `weasyprint` and `markdown`, and the page style is `docs/quickstart-pdf.css`.
- **`docs/README-brown.md`** — the Brown README as it was, kept for the parts
  that are still true: the mode list, the screenshots, the hardware notes.

## Requirements

Rust (stable), and for the SDR backends whatever their own SDKs need —
SoapySDR for the network and USB front ends, the SDRplay API for an RSP, and so
on. The browser client is built with `--target wasm32-unknown-unknown`.
