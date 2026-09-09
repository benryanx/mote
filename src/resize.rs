use crate::Studio;
use eframe::egui;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Canvas,
    Sprite,
}
#[derive(Clone)]
pub struct Resize {
    kind: Kind,
    width: usize,
    height: usize,
    original: (usize, usize),
    anchor: (i32, i32),
    lock_ratio: bool,
}
impl Resize {
    fn offset(&self) -> Option<(i32, i32)> {
        (self.kind == Kind::Canvas).then(|| {
            let dx = self.width as i32 - self.original.0 as i32;
            let dy = self.height as i32 - self.original.1 as i32;
            (dx * self.anchor.0 / 2, dy * self.anchor.1 / 2)
        })
    }
}
impl Studio {
    pub fn open_resize(&mut self, kind: Kind) {
        self.playing = false;
        self.resize = Some(Resize {
            kind,
            width: self.doc.width,
            height: self.doc.height,
            original: (self.doc.width, self.doc.height),
            anchor: (1, 1),
            lock_ratio: true,
        });
    }
    pub fn resize_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut resize) = self.resize.clone() else {
            return;
        };
        let mut close = false;
        egui::Modal::new(egui::Id::new("resize")).show(ctx, |ui| {
            ui.heading(if resize.kind == Kind::Canvas {
                "Canvas Size"
            } else {
                "Sprite Size"
            });
            ui.label(format!(
                "Current: {} × {} px · all {} layers and {} frames",
                resize.original.0,
                resize.original.1,
                self.doc.layers.len(),
                self.doc.frames.len()
            ));
            ui.horizontal(|ui| {
                ui.label("Width");
                if ui
                    .add(
                        egui::DragValue::new(&mut resize.width)
                            .range(1..=2048)
                            .suffix(" px"),
                    )
                    .changed()
                    && resize.kind == Kind::Sprite
                    && resize.lock_ratio
                {
                    resize.height = ((resize.width as f64 * resize.original.1 as f64
                        / resize.original.0 as f64)
                        .round() as usize)
                        .max(1);
                }
                ui.label("Height");
                if ui
                    .add(
                        egui::DragValue::new(&mut resize.height)
                            .range(1..=2048)
                            .suffix(" px"),
                    )
                    .changed()
                    && resize.kind == Kind::Sprite
                    && resize.lock_ratio
                {
                    resize.width = ((resize.height as f64 * resize.original.0 as f64
                        / resize.original.1 as f64)
                        .round() as usize)
                        .max(1);
                }
            });
            if resize.kind == Kind::Sprite {
                ui.checkbox(&mut resize.lock_ratio, "Lock aspect ratio");
                ui.horizontal(|ui| {
                    for (scale, label) in [
                        (0.5, "50%"),
                        (1.0, "100%"),
                        (2.0, "200%"),
                        (3.0, "300%"),
                        (4.0, "400%"),
                    ] {
                        if ui.button(label).clicked() {
                            resize.width =
                                ((resize.original.0 as f64 * scale).round() as usize).max(1);
                            resize.height =
                                ((resize.original.1 as f64 * scale).round() as usize).max(1);
                        }
                    }
                });
                ui.label("Nearest neighbor · keeps pixel edges sharp.");
                ui.small("Whole-number enlargement (200%, 300%…) keeps pixels uniform.");
            } else {
                ui.label("Anchor existing artwork");
                egui::Grid::new("canvas_anchor").show(ui, |ui| {
                    for y in 0..3 {
                        for x in 0..3 {
                            let label = [
                                ["Top left", "Top", "Top right"],
                                ["Left", "Center", "Right"],
                                ["Bottom left", "Bottom", "Bottom right"],
                            ][y as usize][x as usize];
                            ui.selectable_value(&mut resize.anchor, (x, y), label);
                        }
                        ui.end_row();
                    }
                });
                ui.small("Existing pixels keep their size. Added space is transparent.");
                if resize.width < resize.original.0 || resize.height < resize.original.1 {
                    ui.label(
                        "Pixels outside the new canvas will be cropped. Ctrl+Z restores them.",
                    );
                }
            }
            let valid = self.doc.check_size(resize.width, resize.height);
            if let Err(error) = &valid {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(valid.is_ok(), egui::Button::new("Resize"))
                    .clicked()
                {
                    match self
                        .doc
                        .resized(resize.width, resize.height, resize.offset())
                    {
                        Ok(doc) => {
                            if doc != self.doc {
                                self.checkpoint();
                                self.doc = doc;
                                self.selection = None;
                                self.start = None;
                                self.last = None;
                                self.stroke_before = None;
                                self.pan = egui::Vec2::ZERO;
                                self.zoom = 0.0;
                            }
                            self.status =
                                format!("Resized to {} × {} px", resize.width, resize.height);
                            close = true;
                        }
                        Err(error) => self.status = error,
                    }
                }
                if ui.button("Cancel").clicked() {
                    close = true;
                }
            });
        });
        self.resize = if close { None } else { Some(resize) };
    }
}
