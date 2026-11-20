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

The Linux builds and the web client are on the
[releases page](https://github.com/madmedicnl/sdrcrow/releases):

| file | what |
|---|---|
| `sdrcrow-v1.1.0-linux-x86_64.tar.gz` | the standard Linux build — CAT, OpenHPSDR, TCI, RTL-SDR, Airspy, RX-888, ELAD; no SoapySDR needed |
| `sdrcrow-v1.1.0-linux-x86_64.deb` | the same as a Debian package |
| `sdrcrow-v1.1.0-linux-x86_64-soapysdr.deb` | with SoapySDR device support |
| `sdrcrow-v1.1.0-linux-x86_64.AppImage` | one file, run it |
| `sdrcrow-web-v1.1.0.zip` | the browser client, to serve from a web server |
| `SHA256SUMS` | the checksums |

macOS and Windows are not in this release: they need the release workflow,
which is the next piece of work. Everything below builds from source.

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
