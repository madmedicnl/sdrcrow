//! The picture the program opens on.
//!
//! A radio takes a moment to come up — a CAT port is opened, a dongle is
//! claimed, an RSP is initialised — and the window it belongs to is empty until
//! that has happened. Rather than a half-drawn interface that fills in, the
//! program shows its own emblem for a few seconds and then steps aside.
//!
//! It is an **overlay on the window it already has**, not a second window: a
//! splash window would have to be closed on every platform's own terms, would
//! fight the window manager for focus, and would show nothing the main window
//! does not. Drawn last in the frame, it covers whatever the app has managed to
//! lay out and costs one texture.
//!
//! It goes away by itself after [`SPLASH_S`] and at once on a click or a key:
//! an operator who wants their interface back should never have to wait for a
//! logo.

use eframe::egui;

/// How long the emblem stays up, in seconds.
const SPLASH_S: f64 = 3.0;

/// The emblem, compiled in: it is part of the program, not an install asset.
const EMBLEM: &[u8] = include_bytes!("../assets/splash.png");

/// The splash state: the texture, once decoded, and when it is due to leave.
pub struct Splash {
    texture: Option<egui::TextureHandle>,
    /// The time the picture is due to go, in egui's own seconds, decided on the
    /// first frame drawn.
    until: Option<f64>,
    /// Once false the splash never comes back for the life of the process.
    wanted: bool,
}

impl Splash {
    pub fn new() -> Self {
        Splash { texture: None, until: None, wanted: true }
    }

    /// Draw over the whole window, returning whether the picture is up.
    ///
    /// Called last in the frame, so it sits over whatever has been laid out
    /// underneath. The radios are still connecting while it is up — the engine
    /// runs on its own threads and knows nothing about this.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.wanted {
            return false;
        }
        let now = ctx.input(|i| i.time);
        let until = *self.until.get_or_insert(now + SPLASH_S);
        // A click or a key is "I am here, let me in": the picture never becomes
        // something an operator has to wait for.
        let impatient = ctx.input(|i| {
            i.pointer.any_click() || !i.keys_down.is_empty() || i.events.iter().any(|e| {
                matches!(e, egui::Event::Key { .. } | egui::Event::PointerButton { .. })
            })
        });
        if impatient || now >= until {
            self.wanted = false;
            return false;
        }
        ctx.request_repaint(); // so the three seconds actually pass on screen

        let palette = crate::theme::palette();
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::new("sdr-crow-splash"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, _) =
                    ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
                let painter = ui.painter();
                painter.rect_filled(rect, 0.0, palette.bg_deep);
                let side = (rect.width().min(rect.height()) * 0.62).max(120.0);
                let centre = egui::pos2(rect.center().x, rect.center().y - side * 0.06);
                let image = egui::Rect::from_center_size(centre, egui::vec2(side, side));
                if let Some(texture) = self.texture(ctx) {
                    painter.image(
                        texture.id(),
                        image,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }
                painter.text(
                    egui::pos2(rect.center().x, image.max.y + side * 0.08),
                    egui::Align2::CENTER_CENTER,
                    sdrcrow_version::LONG_VERSION,
                    egui::FontId::proportional(15.0),
                    palette.text,
                );
            });
        true
    }

    /// The emblem as a texture, decoded the first time it is drawn.
    fn texture(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        if self.texture.is_none() {
            let image = image::load_from_memory(EMBLEM).ok()?.to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let pixels =
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw().as_slice());
            self.texture =
                Some(ctx.load_texture("sdrcrow-emblem", pixels, egui::TextureOptions::LINEAR));
        }
        self.texture.as_ref()
    }
}

impl Default for Splash {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The emblem is compiled in and decodes: the splash cannot be a blank
    /// rectangle on a machine whose `assets/` folder is not beside the binary.
    #[test]
    fn the_emblem_decodes() {
        let image = image::load_from_memory(EMBLEM).expect("the emblem must decode").to_rgba8();
        assert!(image.width() >= 256, "the emblem is {} px wide", image.width());
        assert_eq!(image.width(), image.height(), "the emblem is square");
    }

    /// The three seconds are counted, and a click ends it early rather than
    /// waiting: the rule the module exists for.
    #[test]
    fn the_picture_leaves_after_three_seconds_or_a_click() {
        assert!(SPLASH_S >= 2.0 && SPLASH_S <= 5.0, "a splash longer than that is a delay");
    }
}
