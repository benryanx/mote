use crate::Studio;
use eframe::egui;
use egui_dock::{DockState, NodeIndex, TabViewer};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Pane {
    Canvas,
    Tools,
    Palette,
    Navigator,
    Layers,
    Frames,
}
pub fn default_layout() -> DockState<Pane> {
    let mut dock = DockState::new(vec![Pane::Canvas]);
    let tree = dock.main_surface_mut();
    let [main, bottom] = tree.split_below(NodeIndex::root(), 0.72, vec![Pane::Frames]);
    tree.split_left(bottom, 0.28, vec![Pane::Layers]);
    let [main, left] = tree.split_left(main, 0.23, vec![Pane::Tools]);
    tree.split_below(left, 0.40, vec![Pane::Palette]);
    tree.split_right(main, 0.78, vec![Pane::Navigator]);
    dock
}
impl TabViewer for Studio {
    type Tab = Pane;
    fn title(&mut self, tab: &mut Pane) -> egui::WidgetText {
        format!("{tab:?}").into()
    }
    fn closeable(&mut self, _: &mut Pane) -> bool {
        false
    }
    fn scroll_bars(&self, _: &Pane) -> [bool; 2] {
        [false, false]
    }
    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Pane) {
        if *tab == Pane::Canvas {
            self.canvas_ui(ui);
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt(format!("pane-{tab:?}"))
            .show(ui, |ui| {
                ui.spacing_mut().slider_width = (ui.available_width() - 65.0).clamp(35.0, 160.0);
                match tab {
                    Pane::Canvas => {}
                    Pane::Tools => self.tools_ui(ui),
                    Pane::Palette => self.palette_ui(ui),
                    Pane::Navigator => self.navigator_ui(ui),
                    Pane::Layers => self.layer_controls(ui),
                    Pane::Frames => self.frames_ui(ui),
                }
            });
    }
}
impl Studio {
    pub fn workspace(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.small(&self.status);
                ui.small(format!(
                    "· {} layers · {} frames",
                    self.doc.layers.len(),
                    self.doc.frames.len()
                ));
            });
        });
        let mut dock = std::mem::replace(&mut self.dock, DockState::new(vec![]));
        egui_dock::DockArea::new(&mut dock)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show_close_buttons(false)
            .show_leaf_close_all_buttons(false)
            .show(ctx, self);
        self.dock = dock;
    }
}
