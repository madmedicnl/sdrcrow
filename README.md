# SDRCrow

<img width="626" height="626" alt="SDRCROW" src="https://github.com/user-attachments/assets/b3e14ac0-f102-4846-bbc3-0bebae4f63e1" />


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

## Where things are

- **`SDRCROW.md`** — what was changed to detach this build, and what is still
  Brown underneath (the i18n domain, the old config variable, the packaging for
  other platforms).
- **`AGENTS.md`** — how work on this tree is done: the gate, the tests, the
  wire, translations, and the rules that keep two people from colliding.
- **`docs/README-brown.md`** — the Brown README as it was, kept for the parts
  that are still true: the mode list, the screenshots, the hardware notes.

## Requirements

Rust (stable), and for the SDR backends whatever their own SDKs need —
SoapySDR for the network and USB front ends, the SDRplay API for an RSP, and so
on. The browser client is built with `--target wasm32-unknown-unknown`.
