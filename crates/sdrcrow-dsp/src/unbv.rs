//! UNBV — **Ultra Narrowband Voice Experimenter**.
//!
//! Recorded speech is slowed by a *factor* and shifted to a narrow slice of the
//! SSB passband: 100–2400 Hz becomes (100–2400)/factor plus a centre, so at the
//! original's default factor 50 the voice occupies about 46 Hz around 1 kHz. The
//! other end shifts it back down and speeds it up. Nothing is gained for free —
//! a message takes `factor` times as long — but the same power then sits in
//! `1/factor` of the bandwidth, which is what the experiment is about.
//!
//! The chain, as the original program does it with ffmpeg filters:
//!
//! * encode: band 100–2400 Hz → slow by the factor (`asetrate` + `aresample`) →
//!   shift up to the centre (`afreqshift`) → level and a limiter;
//! * decode: shift back down → speed up by the factor → band 100–2400 again.
//!
//! Everything here works on a recorded block, never in real time, so the filters
//! may be as long as they like and the delay is simply compensated.

use std::f64::consts::TAU;

/// The speech band the experiment starts from, as the original's chain does.
pub const SPEECH_LO_HZ: f64 = 100.0;
pub const SPEECH_HI_HZ: f64 = 2400.0;

/// Where the slowed speech is put in the SSB passband.
pub const DEFAULT_CENTRE_HZ: f64 = 1000.0;

/// The slowing factor: the original's window starts at 50 and offers the whole
/// range; the ends are ours.
pub const MIN_FACTOR: f64 = 2.0;
pub const MAX_FACTOR: f64 = 100.0;
pub const DEFAULT_FACTOR: f64 = 50.0;

/// The ID interval the operator may set, in minutes — the original interrupts a
/// long transmission every nine to identify itself.
pub const MIN_ID_MINUTES: f64 = 5.0;
pub const MAX_ID_MINUTES: f64 = 15.0;
pub const DEFAULT_ID_MINUTES: f64 = 9.0;

/// One block of speech turned into the narrow signal, or back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnbvSettings {
    /// How much the speech is slowed; the bandwidth falls by the same number.
    pub factor: f64,
    /// Where the slowed speech sits in the passband, in Hz.
    pub centre_hz: f64,
}

impl Default for UnbvSettings {
    fn default() -> Self {
        Self { factor: DEFAULT_FACTOR, centre_hz: DEFAULT_CENTRE_HZ }
    }
}

impl UnbvSettings {
    /// Keep the factor and the centre inside what the chain and the window allow.
    pub fn sane(self) -> Self {
        Self {
            factor: self.factor.clamp(MIN_FACTOR, MAX_FACTOR),
            centre_hz: self.centre_hz.clamp(200.0, 2800.0),
        }
    }

    /// The slice the encoded signal occupies, in Hz — 1002–1048 at the default.
    pub fn band_hz(&self) -> (f64, f64) {
        (
            self.centre_hz + SPEECH_LO_HZ / self.factor,
            self.centre_hz + SPEECH_HI_HZ / self.factor,
        )
    }
}

/// Encode, decode and answer questions about the slice, at one sample rate.
#[derive(Clone, Copy, Debug)]
pub struct Unbv {
    rate: f64,
}

impl Unbv {
    pub fn new(rate: f64) -> Self {
        Self { rate: rate.max(8000.0) }
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Recorded speech in, the narrow signal out. The result is `factor` times
    /// as long as the input, at the same sample rate.
    pub fn encode(&self, speech: &[f32], settings: UnbvSettings) -> Vec<f32> {
        let settings = settings.sane();
        let band = band_limit(speech, SPEECH_LO_HZ, SPEECH_HI_HZ, self.rate);
        let slowed = stretch(&band, settings.factor);
        let moved = shift(&slowed, settings.centre_hz, self.rate);
        normalise(moved)
    }

    /// The narrow signal in, speech out: shifted back, sped up, band-limited and
    /// levelled. About `1/factor` of the input's length.
    pub fn decode(&self, narrow: &[f32], settings: UnbvSettings) -> Vec<f32> {
        let settings = settings.sane();
        let back = shift(narrow, -settings.centre_hz, self.rate);
        let fast = compress(&back, settings.factor);
        let band = band_limit(&fast, SPEECH_LO_HZ, SPEECH_HI_HZ, self.rate);
        normalise(band)
    }
}

/// A windowed-sinc low-pass, applied in one pass with its delay removed.
fn lowpass(signal: &[f32], cutoff_hz: f64, rate: f64) -> Vec<f32> {
    let taps = 201;
    let mid = (taps - 1) as f64 / 2.0;
    let fc = cutoff_hz / rate; // cycles per sample
    let mut kernel: Vec<f64> = (0..taps)
        .map(|i| {
            let n = i as f64 - mid;
            if n.abs() < 1e-9 {
                2.0 * fc
            } else {
                (TAU * fc * n).sin() / (std::f64::consts::PI * n)
            }
        })
        .collect();
    // Blackman window, so the stopband is quiet rather than merely sloping.
    for (i, tap) in kernel.iter_mut().enumerate() {
        let x = i as f64 / (taps - 1) as f64;
        let window = 0.42 - 0.5 * (TAU * x).cos() + 0.08 * (2.0 * TAU * x).cos();
        *tap *= window;
    }
    let gain: f64 = kernel.iter().sum();
    for tap in &mut kernel {
        *tap /= gain;
    }
    let mut out = vec![0.0f32; signal.len()];
    for (n, slot) in out.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for (k, &tap) in kernel.iter().enumerate() {
            let index = n as f64 + k as f64 - mid;
            let i = index.round() as isize;
            if i >= 0 && (i as usize) < signal.len() {
                acc += tap * signal[i as usize] as f64;
            }
        }
        *slot = acc as f32;
    }
    out
}

/// The speech band: 100 Hz high-pass (the input minus its low part) and a 2400
/// Hz low-pass.
fn band_limit(signal: &[f32], lo_hz: f64, hi_hz: f64, rate: f64) -> Vec<f32> {
    let low = lowpass(signal, hi_hz, rate);
    if lo_hz <= 0.0 {
        return low;
    }
    let below = lowpass(&low, lo_hz, rate);
    low.iter().zip(below.iter()).map(|(a, b)| a - b).collect()
}

/// Play the signal `factor` times slower at the same rate: the spectrum is
/// divided by the factor. Cubic interpolation between the original samples is
/// plenty — what is left at that point is a few tens of Hz wide.
fn stretch(signal: &[f32], factor: f64) -> Vec<f32> {
    let out_len = ((signal.len() as f64) * factor).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        out.push(sample_at(signal, i as f64 / factor));
    }
    out
}

/// The other direction: `factor` times faster, `1/factor` of the length.
fn compress(signal: &[f32], factor: f64) -> Vec<f32> {
    let out_len = ((signal.len() as f64) / factor).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        out.push(sample_at(signal, i as f64 * factor));
    }
    out
}

/// Catmull-Rom interpolation at a fractional position.
fn sample_at(signal: &[f32], at: f64) -> f32 {
    if signal.is_empty() {
        return 0.0;
    }
    let base = at.floor();
    let frac = at - base;
    let i = base as isize;
    let get = |k: isize| -> f64 {
        let index = (i + k).clamp(0, signal.len() as isize - 1) as usize;
        signal[index] as f64
    };
    let (p0, p1, p2, p3) = (get(-1), get(0), get(1), get(2));
    let a = -0.5 * p0 + 1.5 * p1 - 1.5 * p2 + 0.5 * p3;
    let b = p0 - 2.5 * p1 + 2.0 * p2 - 0.5 * p3;
    let c = -0.5 * p0 + 0.5 * p2;
    (((a * frac + b) * frac + c) * frac + p1) as f32
}

/// Shift a real signal by `hz` — the analytic signal from a Hilbert transformer,
/// turned by the carrier. A real multiply would give both sidebands, which in an
/// SSB passband would put half the voice below the centre.
fn shift(signal: &[f32], hz: f64, rate: f64) -> Vec<f32> {
    if hz == 0.0 {
        return signal.to_vec();
    }
    let quadrature = hilbert(signal);
    let step = TAU * hz / rate;
    signal
        .iter()
        .zip(quadrature.iter())
        .enumerate()
        .map(|(n, (i, q))| {
            let phase = step * n as f64;
            ((*i as f64) * phase.cos() - (*q as f64) * phase.sin()) as f32
        })
        .collect()
}

/// The imaginary part of the analytic signal: a 127-tap windowed Hilbert FIR,
/// with its delay compensated so the result lines up with the input.
fn hilbert(signal: &[f32]) -> Vec<f32> {
    let taps = 127;
    let mid = (taps - 1) / 2;
    let mut kernel = vec![0.0f64; taps];
    for (i, tap) in kernel.iter_mut().enumerate() {
        let n = i as isize - mid as isize;
        if n == 0 {
            continue;
        }
        let ideal = 2.0 / (std::f64::consts::PI * n as f64);
        let window = 0.42 - 0.5 * (TAU * (i as f64) / (taps - 1) as f64).cos()
            + 0.08 * (2.0 * TAU * (i as f64) / (taps - 1) as f64).cos();
        *tap = (if n % 2 == 0 { 0.0 } else { ideal }) * window;
    }
    let mut out = vec![0.0f32; signal.len()];
    for (n, slot) in out.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for (k, &tap) in kernel.iter().enumerate() {
            let index = n as isize + k as isize - mid as isize;
            if index >= 0 && (index as usize) < signal.len() {
                acc += tap * signal[index as usize] as f64;
            }
        }
        *slot = acc as f32;
    }
    out
}

/// Peak-normalise to −1 dBFS and round the peaks over with a soft limiter.
fn normalise(mut signal: Vec<f32>) -> Vec<f32> {
    let peak = signal.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let gain = 0.891 / peak; // −1 dBFS
        for sample in &mut signal {
            *sample *= gain;
        }
    }
    let ceiling = 0.98;
    for sample in &mut signal {
        let x = *sample as f64 / ceiling as f64;
        *sample = (x.tanh() * ceiling as f64) as f32;
    }
    signal
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn tone(freq: f64, seconds: f64) -> Vec<f32> {
        let n = (seconds * RATE) as usize;
        (0..n).map(|i| (TAU * freq * i as f64 / RATE).sin() as f32 * 0.5).collect()
    }

    /// Magnitude at one frequency (Goertzel), normalised by the sample count.
    fn magnitude(signal: &[f32], freq: f64) -> f64 {
        let k = TAU * freq / RATE;
        let mut s1 = 0.0f64;
        let mut s2 = 0.0f64;
        for &x in signal {
            let s0 = x as f64 + 2.0 * k.cos() * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        ((s1 * s1 + s2 * s2 - 2.0 * k.cos() * s1 * s2).max(0.0)).sqrt()
            / signal.len().max(1) as f64
    }

    /// The strongest frequency in a range, and how strong it is.
    fn peak_in(signal: &[f32], lo: f64, hi: f64, step: f64) -> (f64, f64) {
        let mut best = (lo, 0.0);
        let mut f = lo;
        while f <= hi {
            let m = magnitude(signal, f);
            if m > best.1 {
                best = (f, m);
            }
            f += step;
        }
        best
    }

    #[test]
    fn a_700_hz_tone_lands_at_the_centre_plus_a_fiftieth_of_itself() {
        let unbv = Unbv::new(RATE);
        let settings = UnbvSettings::default();
        let out = unbv.encode(&tone(700.0, 1.0), settings);
        let (hz, level) = peak_in(&out, 1000.0, 1030.0, 0.2);
        assert!(
            (hz - 1014.0).abs() < 1.5,
            "a 700 Hz tone at factor 50 belongs at 1014 Hz, found {hz:.1} Hz (level {level:.3})"
        );
    }

    #[test]
    fn the_band_narrows_by_the_factor() {
        let unbv = Unbv::new(RATE);
        let settings = UnbvSettings::default();
        let mut speech = tone(300.0, 1.0);
        for (i, s) in tone(1800.0, 1.0).iter().enumerate() {
            speech[i] += s;
        }
        let out = unbv.encode(&speech, settings);
        let (low, _) = peak_in(&out, 1000.0, 1020.0, 0.2);
        let (high, _) = peak_in(&out, 1025.0, 1045.0, 0.2);
        // 300 -> 1006 and 1800 -> 1036: thirty Hz apart, as (1800-300)/50.
        assert!(
            (low - 1006.0).abs() < 1.5 && (high - 1036.0).abs() < 1.5,
            "expected 1006 and 1036 Hz, found {low:.1} and {high:.1}"
        );
        assert!(
            (high - low - 30.0).abs() < 2.0,
            "the two tones should be 30 Hz apart, they are {:.1}",
            high - low
        );
    }

    #[test]
    fn nothing_of_the_speech_is_left_outside_its_slice() {
        let unbv = Unbv::new(RATE);
        let settings = UnbvSettings::default();
        let out = unbv.encode(&tone(1200.0, 1.0), settings);
        let (lo, hi) = settings.band_hz();
        // The encoded signal is fifty times as long, so its measurement window is
        // fifty times finer: the tone is at exactly 1000 + 1200/50 and a whole Hz
        // away is a null. Compare it with frequencies well outside the slice.
        let inside = magnitude(&out, 1000.0 + 1200.0 / settings.factor);
        let below = magnitude(&out, 960.0);
        let above = magnitude(&out, 1104.0);
        assert!(
            inside > below * 20.0 && inside > above * 20.0,
            "energy outside the slice ({lo:.0}–{hi:.0} Hz): inside {inside:e}, below {below:e}, above {above:e}"
        );
    }

    #[test]
    fn the_slowed_signal_is_the_factor_longer() {
        let unbv = Unbv::new(RATE);
        let settings = UnbvSettings::default();
        let input = tone(900.0, 0.5);
        let out = unbv.encode(&input, settings);
        let expected = (input.len() as f64 * 50.0) as usize;
        let ratio = out.len() as f64 / expected as f64;
        assert!((ratio - 1.0).abs() < 0.01, "length ratio {ratio:.3}, wanted 1.000");
    }

    #[test]
    fn encoding_then_decoding_gives_the_tone_and_the_length_back() {
        let unbv = Unbv::new(RATE);
        let settings = UnbvSettings::default();
        let input = tone(1200.0, 0.5);
        let narrow = unbv.encode(&input, settings);
        let back = unbv.decode(&narrow, settings);
        let (hz, level) = peak_in(&back, 1150.0, 1250.0, 1.0);
        assert!((hz - 1200.0).abs() < 6.0, "1200 Hz came back at {hz:.1} Hz (level {level:.3})");
        let ratio = back.len() as f64 / input.len() as f64;
        assert!((ratio - 1.0).abs() < 0.05, "length came back at {ratio:.3} of the input");
    }

    #[test]
    fn the_factor_is_kept_inside_what_the_window_offers() {
        let settings = UnbvSettings { factor: 900.0, centre_hz: 10.0 }.sane();
        assert_eq!(settings.factor, MAX_FACTOR);
        assert_eq!(settings.centre_hz, 200.0);
    }

    #[test]
    fn the_slice_is_where_the_shift_puts_it() {
        let (lo, hi) = UnbvSettings::default().band_hz();
        assert!((lo - 1002.0).abs() < 0.01 && (hi - 1048.0).abs() < 0.01, "{lo}..{hi}");
    }
}
