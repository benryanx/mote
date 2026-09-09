use crate::{Studio, palettes, rgba};
use eframe::egui::{self, Sense, Stroke, Vec2};

impl Studio {
    pub fn palette_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Foreground");
            ui.color_edit_button_srgba_unmultiplied(&mut self.color);
            ui.label("Background");
            ui.color_edit_button_srgba_unmultiplied(&mut self.secondary);
        });
        ui.label("Color opacity");
        let mut opacity = self.color[3] as f32 / 255.0 * 100.0;
        if ui
            .add(
                egui::Slider::new(&mut opacity, 0.0..=100.0)
                    .suffix("%")
                    .fixed_decimals(0),
            )
            .changed()
        {
            self.color[3] = (opacity / 100.0 * 255.0).round() as u8;
        }
        ui.monospace(format!(
            "#{:02X}{:02X}{:02X}",
            self.color[0], self.color[1], self.color[2]
        ));
        ui.separator();
        let presets = palettes::presets();
        let label = presets
            .iter()
            .chain(&self.library)
            .find(|p| p.colors == self.doc.palette)
            .map(|p| p.name.as_str())
            .unwrap_or("Custom palette");
        let mut chosen = None;
        egui::ComboBox::from_id_salt("palette_presets")
            .width((ui.available_width() - 16.0).max(60.0))
            .selected_text(label)
            .show_ui(ui, |ui| {
                for p in presets.iter().chain(&self.library) {
                    if ui
                        .selectable_label(p.colors == self.doc.palette, &p.name)
                        .clicked()
                    {
                        chosen = Some(p.clone());
                    }
                }
            });
        if let Some(p) = chosen {
            self.checkpoint();
            self.doc.palette = p.colors;
            self.palette_name = p.name;
        }
        ui.small(format!(
            "{} colors · left: foreground · right: background",
            self.doc.palette.len()
        ));
        ui.spacing_mut().item_spacing.x = 4.0;
        let columns = ((ui.available_width() + 4.0) / 32.0).floor().max(1.0) as usize;
        let mut edit = None;
        egui::Grid::new("swatches")
            .min_col_width(0.0)
            .spacing(Vec2::splat(4.0))
            .show(ui, |ui| {
                for (i, c) in self.doc.palette.clone().iter().enumerate() {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                    let painter = ui.painter();
                    painter.rect_filled(rect, 3.0, egui::Color32::from_gray(90));
                    painter.rect_filled(rect, 3.0, rgba(*c));
                    if *c == self.color {
                        painter.rect_stroke(
                            rect.shrink(1.0),
                            3.0,
                            Stroke::new(2.0_f32, ui.visuals().text_color()),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if response.clicked() {
                        self.color = *c;
                    }
                    if response.secondary_clicked() {
                        self.secondary = *c;
                    }
                    response
                        .on_hover_text(format!(
                            "{} · #{:02X}{:02X}{:02X} · alpha {}",
                            i, c[0], c[1], c[2], c[3]
                        ))
                        .context_menu(|ui| {
                            if ui.button("Replace with foreground").clicked() {
                                edit = Some((i, true));
                                ui.close();
                            }
                            if ui.button("Remove swatch").clicked() {
                                edit = Some((i, false));
                                ui.close();
                            }
                        });
                    if (i + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        if let Some((i, replace)) = edit {
            self.checkpoint();
            if replace {
                self.doc.palette[i] = self.color;
            } else {
                self.doc.palette.remove(i);
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.doc.palette.len() < 256,
                    egui::Button::new("+ Add color"),
                )
                .clicked()
            {
                self.checkpoint();
                self.doc.palette.push(self.color);
            }
            if ui.button("New palette").clicked() {
                self.checkpoint();
                self.doc.palette = vec![self.color];
                self.palette_name = "My palette".into();
            }
        });
        ui.add(
            egui::TextEdit::singleline(&mut self.palette_name)
                .desired_width(ui.available_width())
                .hint_text("Palette name"),
        );
        if ui.button("Save to my palettes").clicked() {
            let name = self.palette_name.trim().to_string();
            let mut library = self.library.clone();
            if library.iter().any(|p| p.name == name) {
                self.status =
                    "That palette name exists. Choose a new name to save a variant.".into();
            } else {
                library.push(palettes::Palette {
                    name,
                    colors: self.doc.palette.clone(),
                });
                match palettes::save(&library) {
                    Ok(()) => {
                        self.library = library;
                        self.status = "Palette saved to your library".into();
                    }
                    Err(e) => self.status = e,
                }
            }
        }
        ui.small(format!(
            "Palette changes keep artwork colors. Undo: {}",
            self.binding_label(crate::keybindings::Command::Undo)
        ));
        ui.separator();
        ui.label("Foreground → background ramp");
        ui.label("Number of colors");
        ui.add(egui::Slider::new(&mut self.ramp_steps, 2..=32));
        let ramp: Vec<_> = (0..self.ramp_steps)
            .map(|i| {
                crate::gradient::blend(
                    self.color,
                    self.secondary,
                    i as f32 / (self.ramp_steps - 1) as f32,
                )
            })
            .collect();
        ui.horizontal_wrapped(|ui| {
            for color in &ramp {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                ui.painter().rect_filled(rect, 2.0, rgba(*color));
            }
        });
        if ui
            .add_enabled(
                self.doc.palette.len() + ramp.len() <= 256,
                egui::Button::new("Add ramp to palette"),
            )
            .clicked()
        {
            self.checkpoint();
            self.doc.palette.extend(ramp);
        }
    }
}
