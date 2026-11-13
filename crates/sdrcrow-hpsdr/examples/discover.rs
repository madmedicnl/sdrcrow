//! Broadcast-scan the LAN for OpenHPSDR devices and print what answers.
//!
//! Run: `cargo run -p sdrcrow-hpsdr --example discover`

use std::time::Duration;

fn main() {
    let timeout = Duration::from_millis(1500);
    println!("scanning for HPSDR devices ({} ms)...", timeout.as_millis());
    let devices = sdrcrow_hpsdr::discover(timeout);
    if devices.is_empty() {
        println!("no devices found");
        return;
    }
    for d in &devices {
        println!(
            "  {ip:<15}  {board:<16}  MAC {mac}  P{proto}{used}{sup}",
            ip = d.ip,
            board = d.board,
            mac = d.mac,
            proto = d.protocol,
            used = if d.in_use { "  [in use]" } else { "" },
            sup = if d.supported() { "" } else { "  [unsupported]" },
        );
    }
}
