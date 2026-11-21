//! The AUX SP window's own lane (`Command::SetZoomView` → `ZoomSpectrum`).
//!
//! A second waterfall is worth having only if it is *sharper* than the wide one
//! it sits beside: the panadapter's frame is pooled across the whole span, so
//! magnifying it draws bigger pixels. This lane mixes the window down and
//! decimates to its own width, which is what makes two carriers sixty hertz
//! apart distinguishable in a window a few kilohertz wide while the wide view
//! shows one blob.
//!
//! What is checked here is the whole promise, end to end: the frame that comes
//! back is the window that was asked for, at the columns asked for, with the
//! two tones where they belong and apart by the right number of hertz; its rows
//! reach the waterfall once the lane is running; and the lane is gone — no more
//! frames at all — once the window is closed.

use std::f64::consts::TAU;
use std::sync::Arc;
use std::time::{Duration, Instant};

use sdrcrow_radio::{Complex32, EngineConfig, EngineHandles, IqSource, Result, start_engine};
use sdrcrow_types::{Command, DeviceCaps, RadioEvent, SpectrumFrame};

const RATE: f64 = 768_000.0;
const CENTER: f64 = 14_100_000.0;
/// Two carriers either side of the centre: four hundred hertz apart, which the
/// panadapter's 4096-point transform over 768 kHz (187 Hz a bin) draws as one
/// lump and a five-kilohertz window separates with room to spare.
const TONE_HZ: f64 = 200.0;
const WINDOW_HZ: f64 = 5_000.0;
const BINS: u16 = 2048;

/// A front end streaming two tones either side of wherever the engine put it.
struct TwoTone {
    phase_a: f64,
    phase_b: f64,
    /// Where the samples in hand are taken — the engine's own centre, not the
    /// test's, because the lane is pointed by an offset from it.
    center_hz: f64,
    /// Where the engine last put it, for the test to wait on.
    landed: std::sync::Arc<std::sync::Mutex<f64>>,
}

impl TwoTone {
    fn new(landed: std::sync::Arc<std::sync::Mutex<f64>>) -> Self {
        TwoTone { phase_a: 0.0, phase_b: 0.0, center_hz: CENTER, landed }
    }
}

impl IqSource for TwoTone {
    fn sample_rate(&self) -> f64 {
        RATE
    }
    fn center_hz(&self) -> f64 {
        self.center_hz
    }
    fn set_center_hz(&mut self, hz: f64) -> Result<()> {
        self.center_hz = hz;
        *self.landed.lock().unwrap() = hz;
        Ok(())
    }
    fn read(&mut self, buf: &mut [Complex32]) -> Result<usize> {
        // Paced like a real front end, so the engine's loop settles instead of
        // spinning at whatever the CPU can do.
        std::thread::sleep(Duration::from_millis(4));
        let n = buf.len().min(8192);
        let step = TAU / RATE;
        for s in &mut buf[..n] {
            let a = self.phase_a;
            let b = self.phase_b;
            // Well under full scale: a clipped tone is a plateau, and a
            // plateau is not what a resolution test measures.
            *s = Complex32::new(
                (a.cos() + b.cos()) as f32 * 0.02,
                (a.sin() + b.sin()) as f32 * 0.02,
            );
            self.phase_a = (self.phase_a + step * TONE_HZ) % TAU;
            self.phase_b = (self.phase_b - step * TONE_HZ) % TAU;
        }
        Ok(n)
    }
    fn describe(&self) -> String {
        "two-tone generator".into()
    }
}

fn caps() -> DeviceCaps {
    DeviceCaps {
        driver: "test".into(),
        label: "test".into(),
        rx_channels: 1,
        sample_rates: vec![RATE],
        freq_ranges_rx: vec![(1_000_000.0, 60_000_000.0)],
        ..DeviceCaps::default()
    }
}

/// The next zoom frame published within `secs`, if one comes.
fn next_frame(h: &mut EngineHandles, secs: f64) -> Option<Arc<SpectrumFrame>> {
    let deadline = Instant::now() + Duration::from_secs_f64(secs);
    while Instant::now() < deadline {
        if h.zoom_spectrum_out.update() {
            let f = h.zoom_spectrum_out.output_buffer();
            if !f.bins.is_empty() {
                return Some(Arc::new(f.clone()));
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    None
}

/// Frames until one has signal in it: a lane holds zeros for as long as its
/// first transform takes, and a frame built from those is the display floor.
fn first_live_frame(h: &mut EngineHandles) -> Arc<SpectrumFrame> {
    // The lane is seeded from the panadapter's picture, so the first frames are
    // that picture pooled — coarse, and exactly what the operator sees one zoom
    // step ago. Give it the time its own transforms need to take over.
    std::thread::sleep(Duration::from_millis(1_500));
    while h.zoom_spectrum_out.update() {}
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(f) = next_frame(h, 0.5)
            && f.bins.iter().any(|&b| b > 40)
        {
            return f;
        }
    }
    panic!("the zoom lane never produced a spectrum");
}

/// Where the two strongest columns are, in hertz, from `lo..hi` of the frame.
fn peaks_hz(f: &SpectrumFrame, lo: f64, hi: f64) -> Vec<f64> {
    let n = f.bins.len();
    let span = f.span_hz;
    let left = f.center_hz - span / 2.0;
    let at = |i: usize| left + (i as f64 + 0.5) / n as f64 * span;
    let mut left_peak = (0usize, 0u8);
    let mut right_peak = (0usize, 0u8);
    for (i, &v) in f.bins.iter().enumerate() {
        if at(i) < lo || at(i) > hi {
            continue;
        }
        if i < n / 2 && v > left_peak.1 {
            left_peak = (i, v);
        }
        if i >= n / 2 && v > right_peak.1 {
            right_peak = (i, v);
        }
    }
    vec![at(left_peak.0), at(right_peak.0)]
}

/// The engine's own centre, from the first state it publishes.
fn settle_center(h: &mut EngineHandles) -> (f64, f64) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        while let Ok(ev) = h.event_rx.try_recv() {
            if let RadioEvent::State(s) = ev
                && s.center_hz > 0.0
                && s.sample_rate > 0.0
            {
                return (s.center_hz, s.sample_rate);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the engine never published its state");
}

fn open_window(h: &EngineHandles, center: f64) {
    h.cmd_tx
        .send(Command::SetZoomView {
            center_hz: center,
            span_hz: WINDOW_HZ,
            bins: BINS,
            rows_per_sec: 25,
        })
        .unwrap();
}

#[test]
fn the_zoom_window_resolves_what_the_wide_one_cannot() {
    let landed = std::sync::Arc::new(std::sync::Mutex::new(CENTER));
    let mut h = start_engine(
        Box::new(TwoTone::new(landed.clone())),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();
    // Let the stream and the published state settle before asking for a window:
    // the lane is built from the rate and pointed by an offset from the
    // engine's own centre, and neither is known until it has published a state.
    let (center, rate) = settle_center(&mut h);
    eprintln!("settled centre={center} rate={rate}");
    open_window(&h, center);

    let f = first_live_frame(&mut h);
    // The window that was asked for, not the panadapter's.
    assert!(
        (f.span_hz - WINDOW_HZ).abs() < WINDOW_HZ * 0.05,
        "span {} is not the {} Hz window asked for",
        f.span_hz,
        WINDOW_HZ
    );
    assert_eq!(f.bins.len(), usize::from(BINS), "the columns asked for");
    assert!(
        (f.center_hz - center).abs() < 1.0,
        "centre {} is not the window's",
        f.center_hz
    );

    // Four hundred hertz apart, and drawn as two carriers: at the
    // panadapter's 187 Hz a bin this is one lump. The lane's analysis window
    // sets how sharply a peak is placed (the transform is finer than the signal
    // it looks at), so the positions are held to a lobe, not to a hertz.
    let p = peaks_hz(&f, center - 1_000.0, center + 1_000.0);
    let spacing = p[1] - p[0];
    assert!(
        (spacing - 2.0 * TONE_HZ).abs() < 60.0,
        "the two carriers are {spacing:.1} Hz apart, not {}",
        2.0 * TONE_HZ
    );
    assert!(
        ((p[0] + p[1]) / 2.0 - center).abs() < 40.0,
        "the pair is not centred on the window: {:?}",
        p
    );

    // Rows reach the waterfall, at a rate the client chose — the window is not
    // a still picture.
    let mut saw_rows = false;
    let mut clocked = false;
    for _ in 0..40 {
        if let Some(f) = next_frame(&mut h, 0.5) {
            clocked |= f.rows_clocked;
            if !f.rows.is_empty() {
                assert_eq!(f.rows.len() % f.bins.len(), 0, "a row is one frame wide");
                saw_rows = true;
                break;
            }
        }
    }
    assert!(clocked, "the window never said it clocks rows");
    assert!(saw_rows, "no waterfall rows came with the frames");

    // Closing the window closes the lane: at most the one frame already in
    // flight, and then nothing at all.
    h.cmd_tx
        .send(Command::SetZoomView {
            center_hz: 0.0,
            span_hz: 0.0,
            bins: 0,
            rows_per_sec: 0,
        })
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    while h.zoom_spectrum_out.update() {}
    let mut after = 0;
    let deadline = Instant::now() + Duration::from_millis(600);
    while Instant::now() < deadline {
        if h.zoom_spectrum_out.update() {
            after += 1;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(after, 0, "the lane kept publishing after the window closed");

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}

#[test]
fn a_window_wider_than_the_stream_is_held_to_it() {
    // A window the front end's own span cannot cover is drawn across as much of
    // it as there is: the operator opened a window, and a window that never
    // draws is worse than one that says the stream is not that wide.
    let landed = std::sync::Arc::new(std::sync::Mutex::new(CENTER));
    let mut h = start_engine(
        Box::new(TwoTone::new(landed.clone())),
        caps(),
        EngineConfig { remember_session: false, ..Default::default() },
    );
    let thread = h.thread.take();
    let (center, _rate) = settle_center(&mut h);
    h.cmd_tx
        .send(Command::SetZoomView {
            center_hz: center,
            span_hz: RATE * 4.0,
            bins: 512,
            rows_per_sec: 20,
        })
        .unwrap();
    let f = first_live_frame(&mut h);
    assert_eq!(f.bins.len(), 512);
    assert!(
        f.span_hz <= RATE * 1.01,
        "a window wider than the stream came back as {} Hz",
        f.span_hz
    );
    assert!((f.center_hz - center).abs() < 1.0);

    drop(h.cmd_tx);
    if let Some(t) = thread {
        let _ = t.join();
    }
}
