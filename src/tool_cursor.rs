use crate::{Studio, Tool};
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

impl Studio {
    /// Canvas-local vector cursor: no platform cursor theme or image assets.
    pub fn paint_tool_cursor(&self, ctx: &egui::Context, painter: &egui::Painter, pos: Pos2) {
        if ctx.input(|i| i.pointer.middle_down()) {
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
            return;
        }
        let tool = if ctx.input(|i| i.modifiers.alt) {
            Tool::Picker
        } else {
            self.tool
        };
        if self.doc.layers[self.layer].locked && !matches!(tool, Tool::Picker | Tool::Select) {
            ctx.set_cursor_icon(egui::CursorIcon::NotAllowed);
            return;
        }
        ctx.set_cursor_icon(egui::CursorIcon::None);
        // The cross is the exact pointer hotspot; the tool badge is offset so
        // it doesn't cover the pixel being edited. Dual strokes work on any art.
        let line = |a: Pos2, b: Pos2| {
            painter.line_segment([a, b], Stroke::new(3.5_f32, Color32::BLACK));
            painter.line_segment([a, b], Stroke::new(1.5_f32, Color32::WHITE));
        };
        for (a, b) in [
            ((-6.0, 0.0), (-2.0, 0.0)),
            ((2.0, 0.0), (6.0, 0.0)),
            ((0.0, -6.0), (0.0, -2.0)),
            ((0.0, 2.0), (0.0, 6.0)),
        ] {
            line(pos + Vec2::new(a.0, a.1), pos + Vec2::new(b.0, b.1));
        }
        let origin = pos + egui::vec2(10.0, 10.0);
        let path = |points: &[(f32, f32)]| {
            for pair in points.windows(2) {
                line(
                    origin + egui::vec2(pair[0].0, pair[0].1),
                    origin + egui::vec2(pair[1].0, pair[1].1),
                );
            }
        };
        match tool {
            Tool::Pencil => {
                path(&[
                    (0., 16.),
                    (2., 10.),
                    (12., 0.),
                    (16., 4.),
                    (6., 14.),
                    (0., 16.),
                ]);
                path(&[(2., 10.), (6., 14.)]);
                path(&[(10., 2.), (14., 6.)]);
            }
            Tool::Eraser => {
                path(&[
                    (0., 10.),
                    (9., 1.),
                    (17., 9.),
                    (10., 16.),
                    (6., 16.),
                    (0., 10.),
                ]);
                path(&[(4., 6.), (12., 14.)]);
                path(&[(5., 18.), (19., 18.)]);
            }
            Tool::Fill => {
                path(&[(1., 8.), (8., 1.), (16., 9.), (9., 16.), (1., 8.)]);
                path(&[(3., 6.), (3., 0.), (9., 0.), (11., 4.)]);
                path(&[(2., 9.), (15., 9.)]);
                path(&[(18., 10.), (16., 15.), (18., 17.), (20., 15.), (18., 10.)]);
            }
            Tool::Picker => {
                path(&[
                    (0., 18.),
                    (2., 12.),
                    (11., 3.),
                    (15., 7.),
                    (6., 16.),
                    (0., 18.),
                ]);
                path(&[(9., 1.), (17., 9.)]);
                path(&[(12., 3.), (15., 0.), (18., 3.), (15., 6.)]);
            }
            Tool::Line => path(&[(0., 17.), (17., 0.)]),
            Tool::Rectangle => path(&[(0., 1.), (18., 1.), (18., 16.), (0., 16.), (0., 1.)]),
            Tool::Ellipse => {
                for stroke in [
                    Stroke::new(3.5_f32, Color32::BLACK),
                    Stroke::new(1.5_f32, Color32::WHITE),
                ] {
                    painter.circle_stroke(origin + egui::vec2(9., 9.), 8., stroke);
                }
            }
            Tool::Select => {
                for n in [0., 7., 14.] {
                    path(&[(n, 0.), (n + 3., 0.)]);
                    path(&[(n, 17.), (n + 3., 17.)]);
                    path(&[(0., n), (0., n + 3.)]);
                    path(&[(17., n), (17., n + 3.)]);
                }
            }
            Tool::Gradient => {
                path(&[(0., 0.), (18., 0.), (18., 16.), (0., 16.), (0., 0.)]);
                for x in [3., 5., 7., 10., 14.] {
                    path(&[(x, 2.), (x, 14.)]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_vector_cursor_and_locked_layers_are_indicated() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        for (tool, _, _) in crate::TOOLS {
            app.tool = tool;
            let output = ctx.run(Default::default(), |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                app.paint_tool_cursor(ctx, &painter, egui::pos2(80., 80.));
            });
            assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::None);
            assert!(!output.shapes.is_empty());
        }
        app.tool = Tool::Pencil;
        app.doc.layers[0].locked = true;
        let output = ctx.run(Default::default(), |ctx| {
            app.paint_tool_cursor(
                ctx,
                &ctx.layer_painter(egui::LayerId::background()),
                egui::pos2(80., 80.),
            );
        });
        assert_eq!(
            output.platform_output.cursor_icon,
            egui::CursorIcon::NotAllowed
        );
        let output = ctx.run(
            egui::RawInput {
                modifiers: egui::Modifiers::ALT,
                ..Default::default()
            },
            |ctx| {
                app.paint_tool_cursor(
                    ctx,
                    &ctx.layer_painter(egui::LayerId::background()),
                    egui::pos2(80., 80.),
                );
            },
        );
        assert_eq!(output.platform_output.cursor_icon, egui::CursorIcon::None);
    }
}
