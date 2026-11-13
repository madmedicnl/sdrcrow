//! The QO-100 narrowband beacon decoder: BPSK-400/AO-40-uncoded telemetry,
//! run off a dedicated downconversion of the raw IQ — the same shape
//! `sdrcrow_skimmer` and `sdrcrow_ism` use for their own narrowband work.
//! Native-only (runs in the engine); see [`bpsk`] for the protocol itself and
//! [`controller`] for how it is driven from the engine's audio-block loop.

pub mod bpsk;
mod controller;
pub mod fec;
pub mod rs;
pub mod rx;

pub use controller::Qo100Controller;
