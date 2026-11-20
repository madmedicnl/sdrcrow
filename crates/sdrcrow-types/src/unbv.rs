//! What the engine tells a screen about a UNBV over.
//!
//! The experiment rides the voice keyer's over but has a buffer of its own, so
//! its progress is its own too: which block of how many is on the air, and the
//! spectrum of the slice it went out in — the receiver's waterfall cannot show a
//! signal a few tens of hertz wide, and this can.

use serde::{Deserialize, Serialize};

/// One look at the experiment: nothing running, or a message on the air.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UnbvStatus {
    /// True while a message is being sent.
    pub active: bool,
    /// Which block of the plan is on the air, counting from 1; 0 when idle.
    pub block: u32,
    /// How many blocks the plan holds.
    pub blocks: u32,
    /// The factor and centre it was sent with, so the panel can show them.
    pub factor: f32,
    pub centre_hz: f32,
    /// The lower and upper edge of the slice, in Hz.
    pub slice_hz: (f32, f32),
    /// The slice's spectrum in dBFS, low frequency first — one snapshot of the
    /// whole message at the moment it was encoded.
    pub bins: Vec<f32>,
}

impl UnbvStatus {
    /// A line for a screen that shows text instead of bars.
    pub fn summary(&self) -> String {
        if !self.active {
            return String::new();
        }
        format!(
            "{}/{}  {:.0} Hz  {:.0}-{:.0} Hz",
            self.block, self.blocks, self.factor, self.slice_hz.0, self.slice_hz.1
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_says_the_block_and_the_slice() {
        let status = UnbvStatus {
            active: true,
            block: 2,
            blocks: 3,
            factor: 50.0,
            centre_hz: 1000.0,
            slice_hz: (1002.0, 1048.0),
            bins: Vec::new(),
        };
        let line = status.summary();
        assert!(line.starts_with("2/3"), "{line}");
        assert!(line.contains("1002-1048"), "{line}");
    }

    #[test]
    fn an_idle_status_says_nothing() {
        assert_eq!(UnbvStatus::default().summary(), "");
    }
}
