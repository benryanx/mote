use crate::Studio;
use eframe::egui;
use mote::document::CLEAR;

#[derive(Clone, Copy)]
pub enum Action {
    Duplicate,
    Blank,
    Delete,
    Left,
    Right,
    Duration(u32),
}
impl Studio {
    pub fn frame_action(&mut self, index: usize, action: Action) {
        if index >= self.doc.frames.len() {
            return;
        }
        match action {
            Action::Duplicate | Action::Blank => {
                if !self
                    .doc
                    .can_grow(self.doc.layers.len(), self.doc.frames.len() + 1)
                {
                    self.status = "Project frame limit reached".into();
                    return;
                }
                self.checkpoint();
                let mut frame = self.doc.frames[index].clone();
                if matches!(action, Action::Blank) {
                    for cel in &mut frame.cels {
                        cel.fill(CLEAR);
                    }
                }
                self.doc.frames.insert(index + 1, frame);
                self.frame = index + 1;
            }
            Action::Delete => {
                if self.doc.frames.len() == 1 {
                    return;
                }
                self.checkpoint();
                self.doc.frames.remove(index);
                self.frame = index.min(self.doc.frames.len() - 1);
            }
            Action::Left | Action::Right => {
                let target = if matches!(action, Action::Left) {
                    index.saturating_sub(1)
                } else {
                    index + 1
                };
                if target == index || target >= self.doc.frames.len() {
                    return;
                }
                self.checkpoint();
                self.doc.frames.swap(index, target);
                self.frame = target;
            }
            Action::Duration(ms) => {
                self.checkpoint();
                self.doc.frames[index].duration = ms.clamp(10, 10000);
                self.frame = index;
            }
        }
        self.playing = false;
    }
    pub fn frames_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if self.playing { "Pause" } else { "Play" })
                .on_hover_text(self.binding_label(crate::keybindings::Command::Play))
                .clicked()
            {
                self.playing = !self.playing;
                self.tick = std::time::Instant::now();
            }
            if ui
                .button("+ Duplicate")
                .on_hover_text(self.binding_label(crate::keybindings::Command::NewFrame))
                .clicked()
            {
                self.frame_action(self.frame, Action::Duplicate);
            }
            if ui.button("+ Blank").clicked() {
                self.frame_action(self.frame, Action::Blank);
            }
            let mut ms = self.doc.frames[self.frame].duration;
            if ui
                .add(
                    egui::DragValue::new(&mut ms)
                        .range(10..=10000)
                        .suffix(" ms"),
                )
                .changed()
            {
                self.frame_action(self.frame, Action::Duration(ms));
            }
            ui.checkbox(&mut self.onion, "Onion skin");
        });
        ui.separator();
        let mut action = None;
        egui::ScrollArea::horizontal()
            .id_salt("frames_strip")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for f in 0..self.doc.frames.len() {
                        let response = ui.add_sized(
                            [72.0, 54.0],
                            egui::Button::new(format!(
                                "{:02}\n{} ms",
                                f + 1,
                                self.doc.frames[f].duration
                            ))
                            .selected(f == self.frame),
                        );
                        if response.clicked() || response.secondary_clicked() {
                            self.frame = f;
                            self.playing = false;
                        }
                        response
                            .on_hover_text("Right-click for frame actions")
                            .context_menu(|ui| {
                                ui.label(format!("Frame {}", f + 1));
                                for (label, a, enabled) in [
                                    (
                                        "Duplicate",
                                        Action::Duplicate,
                                        self.doc.can_grow(
                                            self.doc.layers.len(),
                                            self.doc.frames.len() + 1,
                                        ),
                                    ),
                                    (
                                        "Insert blank after",
                                        Action::Blank,
                                        self.doc.can_grow(
                                            self.doc.layers.len(),
                                            self.doc.frames.len() + 1,
                                        ),
                                    ),
                                    ("Move earlier", Action::Left, f > 0),
                                    ("Move later", Action::Right, f + 1 < self.doc.frames.len()),
                                    ("Delete", Action::Delete, self.doc.frames.len() > 1),
                                ] {
                                    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                        action = Some((f, a));
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                ui.menu_button("Duration", |ui| {
                                    for ms in [50, 80, 100, 150, 200, 500, 1000] {
                                        if ui.button(format!("{ms} ms")).clicked() {
                                            action = Some((f, Action::Duration(ms)));
                                            ui.close();
                                        }
                                    }
                                });
                            });
                    }
                });
            });
        if let Some((index, a)) = action {
            self.frame_action(index, a);
        }
        ui.small("Right-click a frame for options · ← / → step frames");
    }
}
