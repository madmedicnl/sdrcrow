#!/usr/bin/env python3
"""Take the manual's screenshots from the running program.

A manual's pictures are the program as an operator sees it, so they are taken
from the program itself — not from a test's rasteriser — at the size the manual
shows them. The window is a niri column, so the script maximises it, waits for a
frame, and crops nothing: the capture is the screen the program fills.

    tools/manual_shot.py settings-general --tab general
    tools/manual_shot.py settings-radio-cat --tab radio --radio cat
    tools/manual_shot.py 01-main-window              # no --tab: the main window

What a shot needs is in `SHOTS` below: the page, the radio fixture (a backend
and a plausible device, so a picker has something in it), and the picture it
becomes. A shot that needs a live station — a decode, a logbook with contacts, a
waterfall with signals on it — is not in this table on purpose; those are the
ones only the operator can take, and `docs/internal/SCREENSHOT-PLAN.md` lists
them.

The window is drawn in English at 1920x1080 with the desktop layout, whatever
the machine's own language and the operator's own station are: a manual is not
the place for somebody's callsign or their desktop's locale. The call shown is
`N0CALL`, the international placeholder.

Requires niri (for `niri msg`) and grim. Nothing is written into the operator's
own configuration; every shot gets a throw-away directory in /tmp.
"""

from __future__ import annotations

import json
import os
import pathlib
import shutil
import subprocess
import sys
import time

BINARY = pathlib.Path.home() / "sdrcrow/target/release/sdrcrow"
OUT_DIR = pathlib.Path(__file__).resolve().parent.parent / "docs/images"
# Where a shot's throw-away installation lives. Under `~/.config` and named like
# a second installation on purpose: the General page prints the path of the band
# plan folder in the picture, and `/tmp/...` in a manual reads as an accident.
WORK = pathlib.Path.home() / ".config"

# The radio each picture is taken with: `(backend, {json path: value})`, where
# the path is inside `radio.json`. A value of `None` leaves the field alone.
RADIOS = {
    "soapy": ("Soapy", {}),
    "cat": ("Cat", {
        "cat.family": "Yaesu",
        "cat.serial.path": "/dev/ttyUSB0",
        "cat.serial.baud": 38400,
        "radio_audio_in": "USB Audio CODEC (radio)",
        "radio_audio_out": "USB Audio CODEC (rig)",
        "rx_audio_gain_db": 6.0,
    }),
    "usbaudio": ("UsbAudio", {
        "radio_audio_in": "Line (USB Audio Device)",
        "radio_audio_out": "Speakers (USB Audio Device)",
    }),
    "hpsdr": ("Hpsdr", {"hpsdr.manual_ip": "192.168.1.50", "hpsdr.sample_rate_hz": 768000.0}),
    "tci": ("Tci", {"tci.address": "127.0.0.1:50001"}),
    "rtlsdr": ("RtlSdr", {"rtlsdr.sample_rate_hz": 2400000.0}),
    "sdrplay": ("SdrPlay", {"sdrplay.serial": "0000000001"}),
    "smartsdr": ("SmartSdr", {"smartsdr.address": "192.168.1.60"}),
    "plutosdr": ("Pluto", {"pluto.address": "192.168.2.1"}),
}


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def app_window() -> int | None:
    """The id of this program's window, or None while it is still opening."""
    res = run(["niri", "msg", "--json", "windows"])
    if res.returncode != 0:
        raise SystemExit("niri not running: the capture needs `niri msg`")
    for w in json.loads(res.stdout):
        if w.get("app_id") == "sdrcrow":
            return w["id"]
    return None


def wait_for_window(seconds: float = 25.0) -> int:
    deadline = time.time() + seconds
    while time.time() < deadline:
        wid = app_window()
        if wid is not None:
            # One more moment: the first frames are the splash and the layout
            # settling, and a picture of those is not what the manual wants.
            time.sleep(1.2)
            return wid
        time.sleep(0.3)
    raise SystemExit("the window never appeared")


def base_config(dir: pathlib.Path) -> dict:
    """A configuration written by the program itself, then trimmed for a picture.

    Written by the program rather than by hand: the file has a hundred and fifty
    fields and only the handful below are worth touching, so the rest stay
    whatever this build defaults to.
    """
    # The program writes its configuration on the way out, not on the way in:
    # the caller has already run it once and waited for it to close.
    radio = radio_file(dir)
    if radio is None:
        raise SystemExit(f"the program wrote no configuration in {dir}")

    cfg = dir / "config.toml"
    text = cfg.read_text(encoding="utf-8")
    text = text.replace('layout = "Auto"', 'layout = "Desktop"', 1)
    text = text.replace('language = ""', 'language = "en-US"', 1)
    text = text.replace('theme = "Default"', 'theme = "Default"', 1)
    if "config_import_asked" not in text:
        text = text.replace("[ui]", "[ui]\nconfig_import_asked = true", 1)
    else:
        import re
        text = re.sub(r"^config_import_asked = .*$", "config_import_asked = true", text,
                      count=1, flags=re.M)
    # A placeholder call and no grid: a manual is not a station's own logbook.
    text = text.replace('callsign = ""', 'callsign = "N0CALL"', 1)
    cfg.write_text(text, encoding="utf-8")
    return json.loads(radio.read_text(encoding="utf-8"))


def radio_file(dir: pathlib.Path) -> pathlib.Path | None:
    """Where this installation keeps its radio: flat for one, `radio-N/` for more."""
    flat = dir / "radio.json"
    if flat.is_file():
        return flat
    for candidate in sorted(dir.glob("radio-*/radio.json")):
        return candidate
    return None


def set_path(doc: dict, dotted: str, value) -> None:
    node = doc
    keys = dotted.split(".")
    for key in keys[:-1]:
        node = node.setdefault(key, {})
    node[keys[-1]] = value


def prepare_radio(dir: pathlib.Path, radio: dict | None) -> None:
    path = radio_file(dir)
    if path is None:
        raise SystemExit("no radio to set up")
    doc = json.loads(path.read_text(encoding="utf-8"))
    # The two "bands not confirmed yet" banners are a fresh station's greeting,
    # and a picture of the settings window does not need them across the top.
    doc["rx_bands_confirmed"] = True
    doc["tx_bands_confirmed"] = True
    doc["callsign"] = "N0CALL"
    if radio:
        backend, fields = RADIOS[radio]
        doc["backend"] = backend
        for dotted, value in fields.items():
            set_path(doc, dotted, value)
    path.write_text(json.dumps(doc, indent=2), encoding="utf-8")


def take(name: str, tab: str | None, radio: str | None, out: str) -> pathlib.Path:
    dir = WORK / f"sdrcrow-manual-{name}"
    shutil.rmtree(dir, ignore_errors=True)
    dir.mkdir(parents=True)

    env = dict(os.environ, SDRCROW_CONFIG_DIR=str(dir), LANG="en_US.UTF-8")
    env.pop("SDRCROW_SETTINGS_TRACE", None)

    # First run: the program writes a configuration of its own, which is then
    # edited for the picture.
    first = subprocess.Popen([str(BINARY)], env=env, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL, start_new_session=True)
    try:
        wait_for_window()
    finally:
        first.terminate()
        first.wait(timeout=20)
    base_config(dir)
    prepare_radio(dir, radio)

    time.sleep(3.0)  # let the first run's device claim go before asking again
    env2 = dict(env)
    if tab:
        env2["SDRCROW_OPEN_SETTINGS"] = tab
    proc = subprocess.Popen([str(BINARY)], env=env2, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, start_new_session=True)
    try:
        wid = wait_for_window()
        run(["niri", "msg", "action", "focus-window", "--id", str(wid)])
        run(["niri", "msg", "action", "maximize-column"])
        time.sleep(2.5)  # two seconds of waterfall, so the picture is not empty
        shot = dir / "shot.png"
        res = run(["grim", str(shot)])
        if res.returncode != 0:
            raise SystemExit(f"grim: {res.stderr.strip()}")
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.kill()

    target = OUT_DIR / out
    target.parent.mkdir(parents=True, exist_ok=True)
    if out.endswith(".png"):
        shutil.move(str(shot), target)
    else:
        from PIL import Image

        Image.open(shot).convert("RGB").save(target, quality=92, subsampling=1)
        shot.unlink()
    # The installation was for the picture only: the next run makes its own.
    shutil.rmtree(dir, ignore_errors=True)
    print(f"{out}  <- {name}" + (f" [{tab}]" if tab else "") + (f" [{radio}]" if radio else ""))
    return target


# The settings pages the manual illustrates, and the radio each is shown with.
SHOTS: list[tuple[str, str | None, str | None, str]] = [
    # The pages that are the settings window and nothing else. The station
    # behind them is the real one (SoapySDR finds the RSP here), so the
    # panadapter in the picture holds a live waterfall rather than a still.
    ("settings-general", "general", None, "settings-general.jpg"),
    ("settings-ui", "ui", None, "settings-ui.jpg"),
    ("settings-freedv", "freedv", None, "settings-freedv.jpg"),
    ("settings-winlink", "winlink", None, "settings-winlink.jpg"),
    ("settings-spots", "spots", None, "15-settings-spots.jpg"),
    ("settings-uploads", "uploads", None, "16-settings-uploads.jpg"),
]

# **The nine radio pages are not here on purpose.** Each one is a picture of a
# backend's own fields — the RTL-SDR's gain stages, the CAT rig's serial port,
# the Pluto's address — and the radio behind it has to be that backend, or the
# page shows a lie and the panadapter shows the previous source's rubbish. They
# need the hardware on the desk: see `docs/internal/SCREENSHOT-PLAN.md`, which
# names what each one is taken with. Run one with
# `tools/manual_shot.py settings-radio-rtlsdr --tab radio --radio rtlsdr`.


def main(argv: list[str]) -> int:
    if not BINARY.is_file():
        raise SystemExit(f"build it first: {BINARY}")
    wanted = argv[1:] or []
    todo = [s for s in SHOTS if not wanted or s[0] in wanted]
    if not todo:
        print("nothing matched; the names are:")
        for s in SHOTS:
            print("  ", s[0])
        return 2
    for name, tab, radio, out in todo:
        take(name, tab, radio, out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
