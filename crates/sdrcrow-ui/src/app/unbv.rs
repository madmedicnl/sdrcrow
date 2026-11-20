//! The ultra narrowband voice experiment's window.
//!
//! The chip under *Experimental* in the band menu opens it; nothing of it shows
//! otherwise. It is the transmitting side of the experiment: a keyer slot is
//! recorded as usual, and this window sends that slot slowed by a factor and
//! shifted into a narrow slice of the sideband, cut into blocks of at most the
//! identification interval with the station's call in Morse between them.
//!
//! Everything here is a choice made *before* the command — the factor, the
//! centre, the interval, the slot — exactly as the original program's window
//! treats them, so nothing of it needs storing on the server.

use eframe::egui;
use sdrcrow_types::{Command, Mode};

use super::SdrcrowApp;
use crate::i18n::tr;

impl SdrcrowApp {
    /// Draw the window, if the chip has it open.
    pub(in crate::app) fn unbv_window(&mut self, ctx: &egui::Context, cmds: &mut Vec<Command>) {
        if !self.show_unbv {
            return;
        }
        let open = self.tool_window(
            ctx,
            "unbv",
            &tr!("windows-unbv"),
            [640.0, 460.0],
            self.show_unbv,
            |me, ui| me.unbv_body(ui, cmds),
        );
        self.show_unbv = open;
        // The block counter moves a few times a second; the rest of the screen
        // may idle.
        if self.unbv.active {
            crate::repaint::after_ms(ctx, 200);
        }
    }

    fn unbv_body(&mut self, ui: &mut egui::Ui, cmds: &mut Vec<Command>) {
        crate::chrome::window_body_bg(ui);
        let dial = self.state.vfo_a_hz;
        let side = self.unbv_sideband.unwrap_or(Mode::sideband_for(dial));
        let on_sideband = self.state.rx[0].mode.is_sideband();
        let recording = self.voice.recording;

        ui.label(egui::RichText::new(tr!("unbv-hint")).weak().size(11.5));
        ui.add_space(4.0);

        // The two sidebands, with the band's own offer lit when nothing was
        // overridden — the experiment rides a sideband and nothing else.
        ui.horizontal(|ui| {
            ui.label(tr!("unbv-sideband"));
            for mode in [Mode::Lsb, Mode::Usb] {
                let lit = side == mode;
                if crate::chrome::chip_accent(
                    ui,
                    lit,
                    egui::RichText::new(mode.label()).size(11.5),
                    crate::theme::CYAN(),
                    egui::Color32::BLACK,
                )
                .clicked()
                {
                    self.unbv_sideband = Some(mode);
                    cmds.push(Command::SetMode { rx: sdrcrow_types::RxId::Main, mode });
                }
            }
            if !on_sideband {
                ui.label(
                    egui::RichText::new(format!("{} …", self.state.rx[0].mode.label()))
                        .color(crate::theme::ALERT())
                        .size(11.0),
                );
            }
        });
        ui.add_space(2.0);

        ui.add(
            egui::Slider::new(&mut self.unbv_factor, 2.0..=100.0)
                .logarithmic(true)
                .text(tr!("unbv-factor")),
        );
        ui.add(
            egui::Slider::new(&mut self.unbv_centre_hz, 300.0..=2500.0).text(tr!("unbv-centre")),
        );
        ui.add(
            egui::Slider::new(&mut self.unbv_id_minutes, 5.0..=15.0)
                .step_by(1.0)
                .text(tr!("unbv-id-every")),
        );

        let factor = self.unbv_factor.max(1.0) as f64;
        let centre = self.unbv_centre_hz as f64;
        let (lo, hi) = (centre + 100.0 / factor, centre + 2400.0 / factor);
        ui.label(
            egui::RichText::new(tr!("unbv-slice", lo = format!("{lo:.0}"), hi = format!("{hi:.0}")))
                .color(crate::theme::CYAN())
                .size(11.5),
        );
        ui.add_space(4.0);

        // The slot the speech was recorded into, with the keyer's own REC.
        ui.horizontal(|ui| {
            ui.label(tr!("unbv-slot"));
            let mut slot = self.unbv_slot as i32;
            if ui.add(egui::DragValue::new(&mut slot).range(1..=10)).changed() {
                self.unbv_slot = slot.clamp(1, 10) as u8;
            }
            let index = self.unbv_slot.saturating_sub(1);
            let is_rec = recording == Some(index);
            let rec = ui.add_enabled(
                recording.is_none() || is_rec,
                egui::Button::new(if is_rec {
                    tr!("unbv-record-stop")
                } else {
                    tr!("unbv-record")
                }),
            );
            if rec.clicked() {
                cmds.push(Command::VoiceRecord(if is_rec { None } else { Some(index) }));
            }
            let has_audio = self
                .voice
                .slots
                .get(index as usize)
                .is_some_and(|slot| !slot.is_empty());
            if has_audio && !is_rec {
                ui.label(
                    egui::RichText::new("●")
                        .color(crate::theme::CYAN())
                        .size(10.5),
                );
            }
        });

        ui.horizontal(|ui| {
            let ready = on_sideband && !self.unbv.active;
            let send = ui.add_enabled(ready, egui::Button::new(tr!("unbv-send")));
            if send.clicked() {
                cmds.push(Command::UnbvStart {
                    slot: self.unbv_slot.saturating_sub(1),
                    factor: self.unbv_factor,
                    centre_hz: self.unbv_centre_hz,
                    id_minutes: self.unbv_id_minutes,
                });
            }
            let stop = ui.add_enabled(self.unbv.active, egui::Button::new(tr!("unbv-stop")));
            if stop.clicked() {
                cmds.push(Command::UnbvStop);
            }
            if self.unbv.active {
                ui.label(
                    egui::RichText::new(tr!(
                        "unbv-block",
                        n = self.unbv.block,
                        total = self.unbv.blocks
                    ))
                    .color(crate::theme::CYAN())
                    .size(11.5),
                );
            }
        });

        // The slice itself. The receiver's waterfall cannot show a few tens of
        // hertz, which is the whole reason this is drawn here.
        if !self.unbv.bins.is_empty() {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(tr!("unbv-spectrum")).weak().size(11.0),
            );
            unbv_plot(ui, &self.unbv.bins, self.unbv.slice_hz);
        }
    }
}

/// A thin line plot of the slice: one point per bin, −100 dBFS to 0, with the
/// band's two edges written underneath so the picture has numbers.
fn unbv_plot(ui: &mut egui::Ui, bins: &[f32], slice_hz: (f32, f32)) {
    let width = ui.available_width().max(80.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 110.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(90));
    let floor = -100.0f32;
    let ceiling = 0.0f32;
    let to_y = |db: f32| {
        let t = ((db - floor) / (ceiling - floor)).clamp(0.0, 1.0);
        rect.bottom() - t * rect.height()
    };
    let step = rect.width() / (bins.len().max(2) - 1) as f32;
    let points: Vec<egui::Pos2> = bins
        .iter()
        .enumerate()
        .map(|(i, db)| egui::pos2(rect.left() + i as f32 * step, to_y(*db)))
        .collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(1.0, crate::theme::CYAN()),
    ));
    painter.text(
        egui::pos2(rect.left() + 2.0, rect.top() + 2.0),
        egui::Align2::LEFT_TOP,
        format!("{:.0}-{:.0} Hz", slice_hz.0, slice_hz.1),
        egui::FontId::monospace(10.0),
        crate::theme::CYAN_DIM(),
    );
}
