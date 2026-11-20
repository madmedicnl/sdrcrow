//! A UNBV message: a keyer slot slowed and shifted into the narrow slice, cut
//! into blocks with the station's identification between them.
//!
//! The experiment's own program interrupts a long transmission every nine
//! minutes to identify itself, and this does the same: the encoded audio is
//! planned as one timeline — block, CW identification, block, identification —
//! so the transmitter stays up and the operator hears the whole message as one
//! continuous signal. The identification is audio Morse inside the same
//! sideband, not a mode change: a rig keyed through a sound card never switches
//! mode mid-over.
//!
//! Nothing here touches a radio. It turns samples into samples; the engine
//! plays the plan in place of the microphone, exactly as it plays a keyer slot.

use sdrcrow_dsp::unbv::{Unbv, UnbvSettings};

/// Morse speed for the identification — the common 20 wpm, no Farnsworth.
pub const ID_WPM: f32 = 20.0;
/// Sidetone the identification rides on, inside the SSB passband.
pub const ID_TONE_HZ: f64 = 700.0;
/// Silence before and after the identification, so it cannot run into either
/// block's first or last syllable.
pub const ID_MARGIN_S: f64 = 0.4;

/// One planned transmission.
#[derive(Clone, Debug)]
pub struct UnbvPlan {
    /// The whole over: blocks of message with the identification between them,
    /// at the engine's transmit rate.
    pub audio: Vec<f32>,
    /// How many message blocks it holds, for the panel's "block n/m".
    pub blocks: usize,
    /// Where the blocks are, in samples, so progress can be reported.
    pub starts: Vec<usize>,
    /// The slice the message was put in, in Hz.
    pub slice_hz: (f64, f64),
}

impl UnbvPlan {
    pub fn len(&self) -> usize {
        self.audio.len()
    }

    pub fn is_empty(&self) -> bool {
        self.audio.is_empty()
    }

    /// Which block a playback position is in, counting from 1.
    pub fn block_at(&self, position: usize) -> usize {
        self.starts
            .iter()
            .take_while(|&&start| start <= position)
            .count()
            .max(1)
    }
}

/// A UNBV over in flight: the planned message and the cursor through it.
#[derive(Clone, Debug)]
pub struct UnbvTx {
    pub plan: UnbvPlan,
    pub position: usize,
}

/// Encode a recorded slot and plan the whole over around it.
///
/// `id_minutes` is the longest a block may run before the identification is due
/// — the operator's 5 to 15, nine by default. A message shorter than that is a
/// single block and gets the identification at its end, which is what a
/// licensed operator has to do anyway.
pub fn plan(
    speech: &[f32],
    rate: f64,
    settings: UnbvSettings,
    id_minutes: f64,
    callsign: &str,
) -> UnbvPlan {
    let settings = settings.sane();
    if speech.is_empty() {
        return UnbvPlan {
            audio: Vec::new(),
            blocks: 0,
            starts: Vec::new(),
            slice_hz: settings.band_hz(),
        };
    }
    let encoded = Unbv::new(rate).encode(speech, settings);
    plan_encoded(&encoded, rate, id_minutes, callsign, settings.band_hz())
}

/// The same plan for audio that is already narrow — which is what the tests
/// use, because encoding a long message would cost gigabytes of samples.
pub fn plan_encoded(
    encoded: &[f32],
    rate: f64,
    id_minutes: f64,
    callsign: &str,
    slice_hz: (f64, f64),
) -> UnbvPlan {
    if encoded.is_empty() {
        return UnbvPlan { audio: Vec::new(), blocks: 0, starts: Vec::new(), slice_hz };
    }
    let id = identification(callsign, rate);
    let limit = ((id_minutes.clamp(5.0, 15.0) * 60.0) * rate) as usize;
    let limit = limit.max(1);

    let mut audio = Vec::with_capacity(encoded.len() + encoded.len() / 4);
    let mut starts = Vec::new();
    let mut at = 0usize;
    while at < encoded.len() {
        let end = (at + limit).min(encoded.len());
        starts.push(audio.len());
        audio.extend_from_slice(&encoded[at..end]);
        at = end;
        // Identify at every block boundary, including the last one: an over
        // that ends without a call is the one nobody can answer.
        audio.extend_from_slice(&id);
    }
    let blocks = starts.len();
    UnbvPlan { audio, blocks, starts, slice_hz }
}

/// The station's callsign as audio Morse with silence around it, at `rate`.
pub fn identification(callsign: &str, rate: f64) -> Vec<f32> {
    let call = callsign.trim().to_ascii_uppercase();
    if call.is_empty() {
        return Vec::new();
    }
    let margin = (ID_MARGIN_S * rate) as usize;
    let mut tx = sdrcrow_dsp::CwTx::new(rate, ID_TONE_HZ, ID_WPM);
    tx.push_text(&call);
    let mut audio = vec![0.0f32; tx.queued_samples()];
    tx.next_block(&mut audio);
    let mut out = Vec::with_capacity(audio.len() + 2 * margin);
    out.extend(std::iter::repeat_n(0.0f32, margin));
    out.extend_from_slice(&audio);
    out.extend(std::iter::repeat_n(0.0f32, margin));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    fn tone(freq: f64, seconds: f64) -> Vec<f32> {
        let n = (seconds * RATE) as usize;
        (0..n).map(|i| (std::f64::consts::TAU * freq * i as f64 / RATE).sin() as f32 * 0.4).collect()
    }

    #[test]
    fn a_short_message_is_one_block_and_ends_with_the_call() {
        let plan = plan(&tone(900.0, 0.5), RATE, UnbvSettings::default(), 9.0, "PE1ABC");
        assert_eq!(plan.blocks, 1, "half a second of speech is one block");
        assert_eq!(plan.starts, vec![0]);
        let message = plan.len() - identification("PE1ABC", RATE).len();
        assert!(message > 0, "the block itself must be in the plan");
    }

    #[test]
    fn a_long_message_is_cut_where_the_identification_is_due() {
        // Already-narrow audio, twenty minutes of it, at nine minutes a block.
        let encoded = vec![0.05f32; (RATE * 20.0 * 60.0) as usize];
        let plan = plan_encoded(&encoded, RATE, 9.0, "PE1ABC", (1002.0, 1048.0));
        assert_eq!(plan.blocks, 3, "twenty minutes at nine a block is three blocks");
        let id = identification("PE1ABC", RATE).len();
        let block_len = (9.0 * 60.0 * RATE) as usize;
        for pair in plan.starts.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(gap <= block_len + id, "a block ran long: {gap} samples");
            assert!(gap > block_len, "a block was cut short: {gap} samples");
        }
        // Every block is followed by the call, the last one included.
        assert_eq!(plan.len(), encoded.len() + plan.blocks * id);
    }

    #[test]
    fn the_identification_is_morse_of_the_callsign() {
        let id = identification("PE1ABC", RATE);
        assert!(!id.is_empty(), "a call has to make a sound");
        let seconds = id.len() as f64 / RATE;
        // P E 1 A B C at twenty words a minute: a few seconds, and the silence
        // around it is twice the margin.
        assert!(seconds > 1.0 && seconds < 12.0, "{seconds:.2} s of identification");
        let start = id.iter().position(|s| *s != 0.0).expect("morse");
        assert!(start as f64 / RATE >= ID_MARGIN_S - 0.01, "no silence before the call");
        let end = id.iter().rposition(|s| *s != 0.0).expect("morse");
        assert!((id.len() - end) as f64 / RATE >= ID_MARGIN_S - 0.01, "no silence after the call");
    }

    #[test]
    fn an_empty_call_leaves_the_message_alone() {
        let plan = plan(&tone(900.0, 0.2), RATE, UnbvSettings::default(), 9.0, "");
        let plain = Unbv::new(RATE).encode(&tone(900.0, 0.2), UnbvSettings::default());
        assert_eq!(plan.len(), plain.len(), "no call, no identification inserted");
    }

    #[test]
    fn the_plan_says_which_block_is_playing() {
        let encoded = vec![0.05f32; (RATE * 11.0 * 60.0) as usize];
        let plan = plan_encoded(&encoded, RATE, 5.0, "PE1ABC", (1002.0, 1048.0));
        assert_eq!(plan.blocks, 3, "eleven minutes at five a block is three");
        assert_eq!(plan.block_at(0), 1);
        let second = *plan.starts.get(1).expect("a second block");
        assert_eq!(plan.block_at(second), 2);
        let third = *plan.starts.get(2).expect("a third block");
        assert_eq!(plan.block_at(third), 3);
    }
}
