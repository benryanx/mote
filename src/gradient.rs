use crate::Studio;
use eframe::egui;
use mote::document::Pixel;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Shape {
    #[default]
    Linear,
    Radial,
}
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Pattern {
    #[default]
    Smooth,
    Bayer2,
    Bayer4,
    Bayer8,
}
pub fn blend(a: Pixel, b: Pixel, t: f32) -> Pixel {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| (a[i] as f32 * (1.0 - t) + b[i] as f32 * t).round() as u8)
}
fn threshold(x: usize, y: usize, n: usize) -> f32 {
    let mut rank = 0;
    let mut x = x;
    let mut y = y;
    let mut size = n;
    while size > 1 {
        rank = rank * 4 + [[0, 2], [3, 1]][y & 1][x & 1];
        x >>= 1;
        y >>= 1;
        size >>= 1;
    }
    (rank as f32 + 0.5) / (n * n) as f32
}
pub fn sample(a: Pixel, b: Pixel, t: f32, x: usize, y: usize, pattern: Pattern) -> Pixel {
    match pattern {
        Pattern::Smooth => blend(a, b, t),
        _ => {
            let n = match pattern {
                Pattern::Bayer2 => 2,
                Pattern::Bayer4 => 4,
                _ => 8,
            };
            if t.clamp(0.0, 1.0) > threshold(x, y, n) {
                b
            } else {
                a
            }
        }
    }
}
impl Studio {
    pub fn gradient_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.gradient_shape, Shape::Linear, "Linear");
            ui.selectable_value(&mut self.gradient_shape, Shape::Radial, "Radial");
            egui::ComboBox::from_id_salt("gradient_pattern")
                .selected_text(match self.gradient_pattern {
                    Pattern::Smooth => "Smooth",
                    Pattern::Bayer2 => "Bayer 2×2",
                    Pattern::Bayer4 => "Bayer 4×4",
                    Pattern::Bayer8 => "Bayer 8×8",
                })
                .show_ui(ui, |ui| {
                    for (p, label) in [
                        (Pattern::Smooth, "Smooth"),
                        (Pattern::Bayer2, "Bayer 2×2"),
                        (Pattern::Bayer4, "Bayer 4×4"),
                        (Pattern::Bayer8, "Bayer 8×8"),
                    ] {
                        ui.selectable_value(&mut self.gradient_pattern, p, label);
                    }
                });
            ui.label("Drag foreground → background · selection limits fill");
        });
    }
    pub fn draw_gradient(&mut self, a: (i32, i32), b: (i32, i32), first: Pixel) {
        if a == b || self.doc.layers[self.layer].locked {
            return;
        }
        let second = if first == self.color {
            self.secondary
        } else {
            self.color
        };
        let dx = (b.0 - a.0) as f32;
        let dy = (b.1 - a.1) as f32;
        let length2 = dx * dx + dy * dy;
        let (l, t, r, bt) =
            self.bounds()
                .unwrap_or((0, 0, self.doc.width as i32 - 1, self.doc.height as i32 - 1));
        for y in t..=bt {
            for x in l..=r {
                let px = (x - a.0) as f32;
                let py = (y - a.1) as f32;
                let amount = match self.gradient_shape {
                    Shape::Linear => (px * dx + py * dy) / length2,
                    Shape::Radial => (px * px + py * py).sqrt() / length2.sqrt(),
                };
                let c = sample(
                    first,
                    second,
                    amount,
                    x as usize,
                    y as usize,
                    self.gradient_pattern,
                );
                self.doc.put(self.frame, self.layer, x, y, c);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gradient_respects_selection_lock_and_radial_distance() {
        let mut app = Studio {
            color: [255, 0, 0, 255],
            secondary: [0, 0, 255, 255],
            selection: Some(((1, 1), (3, 3))),
            ..Default::default()
        };
        app.draw_gradient((1, 1), (3, 1), app.color);
        assert_eq!(app.doc.frames[0].cels[0][0], [0; 4]);
        assert_eq!(app.doc.frames[0].cels[0][33], app.color);
        assert_eq!(app.doc.frames[0].cels[0][34], [128, 0, 128, 255]);
        assert_eq!(app.doc.frames[0].cels[0][35], app.secondary);
        app.gradient_shape = Shape::Radial;
        app.draw_gradient((1, 1), (3, 1), app.color);
        assert_eq!(app.doc.frames[0].cels[0][97], app.secondary);
        let before = app.doc.clone();
        app.doc.layers[0].locked = true;
        app.color = [255; 4];
        app.draw_gradient((1, 1), (3, 1), app.color);
        assert_eq!(app.doc.frames, before.frames);
    }
    #[test]
    fn smooth_preserves_endpoints_and_alpha() {
        assert_eq!(blend([0; 4], [255; 4], 0.0), [0; 4]);
        assert_eq!(blend([0; 4], [255; 4], 1.0), [255; 4]);
        assert_eq!(blend([0; 4], [255; 4], 0.5), [128; 4]);
    }
    #[test]
    fn dither_uses_only_endpoints_and_balances_halfway() {
        for (p, n) in [
            (Pattern::Bayer2, 2),
            (Pattern::Bayer4, 4),
            (Pattern::Bayer8, 8),
        ] {
            let mut b = 0;
            for y in 0..n {
                for x in 0..n {
                    let c = sample([0; 4], [255; 4], 0.5, x, y, p);
                    assert!(c == [0; 4] || c == [255; 4]);
                    b += usize::from(c == [255; 4]);
                    assert_eq!(sample([0; 4], [255; 4], 0.0, x, y, p), [0; 4]);
                    assert_eq!(sample([0; 4], [255; 4], 1.0, x, y, p), [255; 4]);
                }
            }
            assert_eq!(b, n * n / 2);
        }
    }
}
