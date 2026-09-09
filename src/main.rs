mod background;
mod documents;
mod exporting;
mod frames;
mod gradient;
mod keybindings;
mod palette_ui;
mod palettes;
mod resize;
mod theme;
mod workspace;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use mote::document::{CLEAR, Document, Pixel, line};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
    Picker,
    Line,
    Rectangle,
    Ellipse,
    Select,
    Gradient,
}
const TOOLS: [(Tool, &str, egui::Key); 9] = [
    (Tool::Gradient, "D  Gradient", egui::Key::D),
    (Tool::Pencil, "B  Pencil", egui::Key::B),
    (Tool::Eraser, "E  Eraser", egui::Key::E),
    (Tool::Fill, "G  Fill", egui::Key::G),
    (Tool::Picker, "I  Eyedropper", egui::Key::I),
    (Tool::Line, "L  Line", egui::Key::L),
    (Tool::Rectangle, "U  Rectangle", egui::Key::U),
    (Tool::Ellipse, "O  Ellipse", egui::Key::O),
    (Tool::Select, "M  Selection", egui::Key::M),
];
#[derive(Clone, Copy)]
enum Pending {
    New,
    Open,
    Close,
    CloseTab,
}
struct Studio {
    io_job: Option<background::Job>,
    bindings: Vec<keybindings::Binding>,
    key_editor: keybindings::Editor,
    export_options: exporting::Options,
    export_open: bool,
    documents: Vec<Option<documents::Session>>,
    active_document: usize,
    quit_reviewed: Vec<usize>,
    title: String,
    document_id: u64,
    next_document_id: u64,
    gradient_shape: gradient::Shape,
    gradient_pattern: gradient::Pattern,
    ramp_steps: usize,
    resize: Option<resize::Resize>,
    dock: egui_dock::DockState<workspace::Pane>,
    palette_name: String,
    library: Vec<palettes::Palette>,
    doc: Document,
    saved: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    frame: usize,
    layer: usize,
    tool: Tool,
    color: Pixel,
    secondary: Pixel,
    brush: i32,
    mirror_x: bool,
    mirror_y: bool,
    filled: bool,
    grid: bool,
    onion: bool,
    tiled: bool,
    zoom: f32,
    pan: Vec2,
    start: Option<(i32, i32)>,
    last: Option<(i32, i32)>,
    stroke_before: Option<Document>,
    selection: Option<((i32, i32), (i32, i32))>,
    clipboard: Option<(usize, usize, Vec<Pixel>)>,
    path: Option<PathBuf>,
    status: String,
    theme: theme::Theme,
    playing: bool,
    tick: Instant,
    new_dialog: bool,
    new_width: usize,
    new_height: usize,
    pending: Option<Pending>,
    allow_close: bool,
    texture: Option<egui::TextureHandle>,
    preview: Option<egui::TextureHandle>,
    recovery_tick: Instant,
}
impl Default for Studio {
    fn default() -> Self {
        let doc = Document::new(32, 32);
        Self {
            io_job: None,
            bindings: keybindings::defaults(),
            key_editor: Default::default(),
            export_options: Default::default(),
            export_open: false,
            documents: vec![None],
            active_document: 0,
            quit_reviewed: vec![],
            title: "Untitled".into(),
            document_id: 1,
            next_document_id: 2,
            gradient_shape: Default::default(),
            gradient_pattern: Default::default(),
            ramp_steps: 8,
            resize: None,
            dock: workspace::default_layout(),
            palette_name: "My palette".into(),
            library: Vec::new(),
            saved: doc.clone(),
            doc,
            undo: vec![],
            redo: vec![],
            frame: 0,
            layer: 0,
            tool: Tool::Pencil,
            color: [165, 199, 120, 255],
            secondary: [23, 28, 44, 255],
            brush: 1,
            mirror_x: false,
            mirror_y: false,
            filled: false,
            grid: false,
            onion: false,
            tiled: false,
            zoom: 14.0,
            pan: Vec2::ZERO,
            start: None,
            last: None,
            stroke_before: None,
            selection: None,
            clipboard: None,
            path: None,
            status: "Ready · Make something small and wonderful".into(),
            theme: Default::default(),
            playing: false,
            tick: Instant::now(),
            new_dialog: false,
            new_width: 32,
            new_height: 32,
            pending: None,
            allow_close: false,
            texture: None,
            preview: None,
            recovery_tick: Instant::now(),
        }
    }
}
impl Studio {
    fn dirty(&self) -> bool {
        self.doc != self.saved
    }
    fn checkpoint(&mut self) {
        self.push_undo(self.doc.clone());
    }
    fn push_undo(&mut self, doc: Document) {
        self.undo.push(doc);
        self.redo.clear();
        let bytes =
            self.doc.width * self.doc.height * self.doc.layers.len() * self.doc.frames.len() * 4;
        let cap = (64 * 1024 * 1024 / bytes.max(1)).clamp(1, 100);
        if self.undo.len() > cap {
            self.undo.drain(..self.undo.len() - cap);
        }
    }
    fn normalize(&mut self) {
        self.layer = self.layer.min(self.doc.layers.len() - 1);
        self.frame = self.frame.min(self.doc.frames.len() - 1);
        self.selection = None;
    }
    fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, d));
            self.normalize();
        }
    }
    fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, d));
            self.normalize();
        }
    }
    fn replace(&mut self, doc: Document, path: Option<PathBuf>) {
        self.doc = doc;
        self.saved = self.doc.clone();
        self.path = path;
        self.undo.clear();
        self.redo.clear();
        self.frame = 0;
        self.layer = 0;
        self.selection = None;
        self.pan = Vec2::ZERO;
        self.playing = false;
    }
    fn request(&mut self, p: Pending, ctx: &egui::Context) {
        if matches!(p, Pending::New | Pending::Open) {
            self.perform(p, ctx);
            return;
        }
        self.finish_gesture();
        if matches!(p, Pending::Close) {
            if let Some(index) = self.next_unsaved_document() {
                self.switch_document(index);
                self.pending = Some(p);
            } else {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        if self.dirty() {
            self.pending = Some(p);
        } else {
            self.perform(p, ctx);
        }
    }
    fn perform(&mut self, p: Pending, ctx: &egui::Context) {
        match p {
            Pending::New => self.new_dialog = true,
            Pending::Open => self.open(),
            Pending::Close => {
                self.quit_reviewed.push(self.active_document);
                self.request(Pending::Close, ctx);
            }
            Pending::CloseTab => self.close_active_document(),
        }
    }
    fn save(&mut self, save_as: bool) -> bool {
        if self.io_job.is_some() {
            return false;
        }
        let doc = self.doc.clone();
        let document_id = self.document_id;
        let existing = if save_as { None } else { self.path.clone() };
        let filename = self.filename("mote");
        self.start_background("Saving project", move || {
            let path = existing.or_else(|| {
                rfd::FileDialog::new()
                    .add_filter("Mote project", &["mote"])
                    .set_file_name(filename)
                    .save_file()
            });
            let Some(path) = path else {
                return Ok(background::Outcome::Canceled);
            };
            doc.save(&path)?;
            Ok(background::Outcome::Saved {
                document_id,
                doc,
                path,
            })
        });
        false
    }
    fn open(&mut self) {
        self.start_background("Opening images", || {
            let Some(paths) = rfd::FileDialog::new()
                .add_filter("Mote / PNG", &["mote", "png"])
                .pick_files()
            else {
                return Ok(background::Outcome::Canceled);
            };
            let files = paths
                .into_iter()
                .map(|path| {
                    let png = path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
                    let result = if png {
                        Self::import_png(&path)
                    } else {
                        Document::load(&path)
                    };
                    (path, result)
                })
                .collect();
            Ok(background::Outcome::Opened(files))
        });
    }
    fn import_png(path: &std::path::Path) -> Result<Document, String> {
        let reader = image::ImageReader::open(path).map_err(|e| e.to_string())?;
        let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
        if w == 0 || h == 0 || w > 2048 || h > 2048 {
            return Err("PNG must be between 1 and 2048 pixels per side".into());
        }
        let img = image::open(path).map_err(|e| e.to_string())?.into_rgba8();
        let mut doc = Document::new(w as usize, h as usize);
        doc.frames[0].cels[0] = img.pixels().map(|p| p.0).collect();
        Ok(doc)
    }
    fn export(&mut self, kind: u8) {
        self.export_options.format = if kind == 2 {
            exporting::Format::Gif
        } else {
            exporting::Format::Png
        };
        self.export_options.mode = match kind {
            1 => exporting::Mode::Sheet,
            2 => exporting::Mode::Animation,
            _ => exporting::Mode::Image,
        };
        self.export_options.scope = if kind == 0 {
            exporting::Scope::Current
        } else {
            exporting::Scope::All
        };
        self.open_export();
    }
    #[cfg(test)]
    fn export_to(&self, kind: u8, path: &std::path::Path) -> Result<(), String> {
        let options = exporting::Options {
            format: if kind == 2 {
                exporting::Format::Gif
            } else {
                exporting::Format::Png
            },
            mode: match kind {
                1 => exporting::Mode::Sheet,
                2 => exporting::Mode::Animation,
                _ => exporting::Mode::Image,
            },
            scope: if kind == 0 {
                exporting::Scope::Current
            } else {
                exporting::Scope::All
            },
            ..Default::default()
        };
        exporting::write(&self.doc, self.frame, self.layer, &options, path).map(|_| ())
    }
    fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        self.selection
            .map(|(a, b)| (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)))
    }
    fn stamp(&mut self, p: (i32, i32), color: Pixel) {
        for dy in 0..self.brush {
            for dx in 0..self.brush {
                let x = p.0 + dx - self.brush / 2;
                let y = p.1 + dy - self.brush / 2;
                let xs = [x, self.doc.width as i32 - 1 - x];
                let ys = [y, self.doc.height as i32 - 1 - y];
                for &x in xs.iter().take(if self.mirror_x { 2 } else { 1 }) {
                    for &y in ys.iter().take(if self.mirror_y { 2 } else { 1 }) {
                        if self
                            .bounds()
                            .is_none_or(|(l, t, r, b)| x >= l && x <= r && y >= t && y <= b)
                        {
                            self.doc.put(self.frame, self.layer, x, y, color);
                        }
                    }
                }
            }
        }
    }
    fn draw(&mut self, a: (i32, i32), b: (i32, i32), color: Pixel) {
        match self.tool {
            Tool::Gradient => self.draw_gradient(a, b, color),
            Tool::Pencil | Tool::Eraser | Tool::Line => {
                for p in line(a, b) {
                    self.stamp(p, color);
                }
            }
            Tool::Rectangle => {
                let (l, r, t, bt) = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
                for y in t..=bt {
                    for x in l..=r {
                        if self.filled || x == l || x == r || y == t || y == bt {
                            self.stamp((x, y), color);
                        }
                    }
                }
            }
            Tool::Ellipse => {
                let cx = (a.0 + b.0) as f32 / 2.0;
                let cy = (a.1 + b.1) as f32 / 2.0;
                let rx = ((a.0 - b.0).abs() as f32 / 2.0).max(0.5);
                let ry = ((a.1 - b.1).abs() as f32 / 2.0).max(0.5);
                for y in a.1.min(b.1)..=a.1.max(b.1) {
                    for x in a.0.min(b.0)..=a.0.max(b.0) {
                        let d = ((x as f32 - cx) / rx).powi(2) + ((y as f32 - cy) / ry).powi(2);
                        let inside = d <= 1.15;
                        let edge = [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)].iter().any(
                            |&(u, v)| {
                                ((u as f32 - cx) / rx).powi(2) + ((v as f32 - cy) / ry).powi(2)
                                    > 1.15
                            },
                        );
                        if inside && (self.filled || edge) {
                            self.stamp((x, y), color);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn copy(&mut self) {
        let (l, t, r, b) =
            self.bounds()
                .unwrap_or((0, 0, self.doc.width as i32 - 1, self.doc.height as i32 - 1));
        let mut pixels = vec![];
        for y in t..=b {
            for x in l..=r {
                pixels.push(
                    self.doc.frames[self.frame].cels[self.layer]
                        [y as usize * self.doc.width + x as usize],
                );
            }
        }
        self.clipboard = Some(((r - l + 1) as usize, (b - t + 1) as usize, pixels));
        self.status = "Selection copied".into();
    }
    fn paste(&mut self) {
        if self.doc.layers[self.layer].locked {
            return;
        }
        if let Some((w, h, pixels)) = self.clipboard.clone() {
            self.checkpoint();
            let (x, y) = self.bounds().map(|(l, t, _, _)| (l, t)).unwrap_or((0, 0));
            for dy in 0..h {
                for dx in 0..w {
                    self.doc.put(
                        self.frame,
                        self.layer,
                        x + dx as i32,
                        y + dy as i32,
                        pixels[dy * w + dx],
                    );
                }
            }
        }
    }
    fn delete_selection(&mut self) {
        if self.doc.layers[self.layer].locked {
            return;
        }
        self.checkpoint();
        let (l, t, r, b) =
            self.bounds()
                .unwrap_or((0, 0, self.doc.width as i32 - 1, self.doc.height as i32 - 1));
        for y in t..=b {
            for x in l..=r {
                self.doc.put(self.frame, self.layer, x, y, CLEAR);
            }
        }
    }
    fn flip(&mut self, horizontal: bool) {
        if self.doc.layers[self.layer].locked {
            return;
        }
        self.checkpoint();
        let (l, t, r, b) =
            self.bounds()
                .unwrap_or((0, 0, self.doc.width as i32 - 1, self.doc.height as i32 - 1));
        let original = self.doc.frames[self.frame].cels[self.layer].clone();
        for y in t..=b {
            for x in l..=r {
                let (sx, sy) = if horizontal {
                    (r - (x - l), y)
                } else {
                    (x, b - (y - t))
                };
                self.doc.put(
                    self.frame,
                    self.layer,
                    x,
                    y,
                    original[sy as usize * self.doc.width + sx as usize],
                );
            }
        }
    }
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("mote").size(25.0).strong());
                ui.separator();
                ui.menu_button("File", |ui| {
                    if ui
                        .button(self.command_label(keybindings::Command::Export, "Export As…"))
                        .clicked()
                    {
                        self.open_export();
                        ui.close();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Close, "Close document"))
                        .clicked()
                    {
                        self.request(Pending::CloseTab, ctx);
                        ui.close();
                    }
                    if ui
                        .button(
                            self.command_label(keybindings::Command::CanvasSize, "Canvas Size…"),
                        )
                        .clicked()
                    {
                        self.open_resize(resize::Kind::Canvas);
                        ui.close();
                    }
                    if ui
                        .button(
                            self.command_label(keybindings::Command::SpriteSize, "Sprite Size…"),
                        )
                        .clicked()
                    {
                        self.open_resize(resize::Kind::Sprite);
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .button(self.command_label(keybindings::Command::New, "New…"))
                        .clicked()
                    {
                        self.request(Pending::New, ctx);
                        ui.close();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Open, "Open…"))
                        .clicked()
                    {
                        self.request(Pending::Open, ctx);
                        ui.close();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Save, "Save"))
                        .clicked()
                    {
                        self.save(false);
                        ui.close();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::SaveAs, "Save as…"))
                        .clicked()
                    {
                        self.save(true);
                        ui.close();
                    }
                    ui.separator();
                    for (kind, label) in ["Export PNG…", "Export sprite sheet…", "Export GIF…"]
                        .iter()
                        .enumerate()
                    {
                        if ui.button(*label).clicked() {
                            self.export(kind as u8);
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .button(
                            self.command_label(
                                keybindings::Command::Shortcuts,
                                "Keyboard Shortcuts…",
                            ),
                        )
                        .clicked()
                    {
                        self.key_editor.open = true;
                        self.playing = false;
                        ui.close();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Undo, "Undo"))
                        .clicked()
                    {
                        self.undo();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Redo, "Redo"))
                        .clicked()
                    {
                        self.redo();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Copy, "Copy selection"))
                        .clicked()
                    {
                        self.copy();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::Paste, "Paste selection"))
                        .clicked()
                    {
                        self.paste();
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::FlipH, "Flip horizontal"))
                        .clicked()
                    {
                        self.flip(true);
                    }
                    if ui
                        .button(self.command_label(keybindings::Command::FlipV, "Flip vertical"))
                        .clicked()
                    {
                        self.flip(false);
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Reset workspace layout").clicked() {
                        self.dock = workspace::default_layout();
                        ui.close();
                    }
                    ui.label("Drag panel tabs to move; drag dividers to resize.");
                    ui.checkbox(&mut self.grid, "Pixel grid");
                    ui.checkbox(&mut self.onion, "Onion skin");
                    ui.checkbox(&mut self.tiled, "Tile preview");
                    if ui.button("Center canvas").clicked() {
                        self.pan = Vec2::ZERO;
                    }
                    if ui.button("Keyboard guide").clicked() {
                        self.key_editor.open = true;
                    }
                });
                ui.separator();
                let name = self
                    .path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or(self.title.clone());
                ui.label(format!("{name}{}", if self.dirty() { " •" } else { "" }));
            });
            ui.separator();
            if self.tool == Tool::Gradient {
                self.gradient_ui(ui);
            }
            ui.horizontal(|ui| {
                ui.label(format!("{:?}", self.tool));
                ui.add(
                    egui::Slider::new(&mut self.brush, 1..=16)
                        .text("px")
                        .show_value(true),
                );
                ui.separator();
                ui.checkbox(&mut self.mirror_x, "Mirror X");
                ui.checkbox(&mut self.mirror_y, "Mirror Y");
                if matches!(self.tool, Tool::Rectangle | Tool::Ellipse) {
                    ui.checkbox(&mut self.filled, "Filled");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Fit").clicked() {
                        self.zoom = 0.0;
                        self.pan = Vec2::ZERO;
                    }
                    ui.label(format!("{}%", (self.zoom * 100.0) as i32));
                });
            });
        });
    }
    fn tools_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                .on_hover_text(self.binding_label(keybindings::Command::Undo))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                .on_hover_text(self.binding_label(keybindings::Command::Redo))
                .clicked()
            {
                self.redo();
            }
        });
        ui.add_space(8.0);
        ui.label(egui::RichText::new("TOOLS").small().weak());
        for (tool, _, _) in TOOLS {
            let label = self.command_label(keybindings::tool_command(tool), &format!("{tool:?}"));
            ui.selectable_value(&mut self.tool, tool, label);
        }
        ui.add_space(16.0);
        ui.separator();
    }
    fn navigator_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        ui.add_space(8.0);
        ui.label(egui::RichText::new("NAVIGATOR").small().weak());
        let pixels = self.doc.composite(self.frame);
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [self.doc.width, self.doc.height],
            &pixels.into_iter().flatten().collect::<Vec<_>>(),
        );
        let tex = self.preview.get_or_insert_with(|| {
            ctx.load_texture("preview", image.clone(), egui::TextureOptions::NEAREST)
        });
        tex.set(image, egui::TextureOptions::NEAREST);
        let scale =
            (ui.available_width() / self.doc.width as f32).min(160.0 / self.doc.height as f32);
        ui.image((
            tex.id(),
            Vec2::new(
                self.doc.width as f32 * scale,
                self.doc.height as f32 * scale,
            ),
        ));
        ui.small(format!("{} × {} · RGBA", self.doc.width, self.doc.height));
    }
    fn layer_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("LAYERS").small().weak());
            if ui
                .small_button("+")
                .on_hover_text(self.command_label(keybindings::Command::NewLayer, "New layer"))
                .clicked()
            {
                self.add_layer(false);
            }
            if ui
                .small_button("Duplicate")
                .on_hover_text(self.binding_label(keybindings::Command::DuplicateLayer))
                .clicked()
            {
                self.add_layer(true);
            }
            if ui
                .small_button("−")
                .on_hover_text(
                    self.command_label(keybindings::Command::DeleteLayer, "Delete layer"),
                )
                .clicked()
            {
                self.delete_layer();
            }
        });
        for l in (0..self.doc.layers.len()).rev() {
            ui.horizontal(|ui| {
                let mut visible = self.doc.layers[l].visible;
                if ui.checkbox(&mut visible, "").changed() {
                    self.checkpoint();
                    self.doc.layers[l].visible = visible;
                }
                if ui
                    .selectable_label(l == self.layer, &self.doc.layers[l].name)
                    .clicked()
                {
                    self.layer = l;
                }
                let mut locked = self.doc.layers[l].locked;
                if ui
                    .toggle_value(&mut locked, "L")
                    .on_hover_text("Lock layer")
                    .changed()
                {
                    self.checkpoint();
                    self.doc.layers[l].locked = locked;
                }
            });
        }
        ui.separator();
        let mut name = self.doc.layers[self.layer].name.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(ui.available_width()))
            .changed()
        {
            self.checkpoint();
            self.doc.layers[self.layer].name = name;
        }
        let mut opacity = self.doc.layers[self.layer].opacity;
        ui.label("Layer opacity");
        if ui.add(egui::Slider::new(&mut opacity, 0..=255)).changed() {
            self.checkpoint();
            self.doc.layers[self.layer].opacity = opacity;
        }
        ui.horizontal(|ui| {
            for (up, label) in [(true, "Raise"), (false, "Lower")] {
                if ui.button(label).clicked() {
                    let target = if up {
                        self.layer + 1
                    } else {
                        self.layer.saturating_sub(1)
                    };
                    if target < self.doc.layers.len() && target != self.layer {
                        self.checkpoint();
                        self.doc.layers.swap(self.layer, target);
                        for f in &mut self.doc.frames {
                            f.cels.swap(self.layer, target);
                        }
                        self.layer = target;
                    }
                }
            }
        });
    }
    fn canvas_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        if self.zoom == 0.0 {
            self.zoom = ((area.width() - 64.0) / self.doc.width as f32)
                .min((area.height() - 64.0) / self.doc.height as f32)
                .clamp(0.25, 64.0);
        }
        if response.hovered() {
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let old = self.zoom;
                self.zoom = (self.zoom * (scroll * 0.003).exp()).clamp(0.25, 64.0);
                if let Some(pos) = response.hover_pos() {
                    let offset = area.center() - pos;
                    self.pan = (self.pan + offset) * (self.zoom / old) - offset;
                }
            }
        }
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += ctx.input(|i| i.pointer.delta());
        }
        let size = Vec2::new(self.doc.width as f32, self.doc.height as f32) * self.zoom;
        let rect = Rect::from_center_size(area.center() + self.pan, size);
        let painter = ui.painter_at(area);
        // Checkerboard is screen-space and clipped, keeping large canvases inexpensive.
        let visible = rect.intersect(area);
        if visible.is_positive() {
            let cell = (self.zoom * 4.0).max(8.0);
            let cols = (visible.width() / cell).ceil() as i32;
            let rows = (visible.height() / cell).ceil() as i32;
            for y in 0..rows {
                for x in 0..cols {
                    let r = Rect::from_min_size(
                        visible.min + Vec2::new(x as f32 * cell, y as f32 * cell),
                        Vec2::splat(cell),
                    )
                    .intersect(visible);
                    painter.rect_filled(
                        r,
                        0.0,
                        Color32::from_gray(if (x + y) % 2 == 0 { 43 } else { 51 }),
                    );
                }
            }
        }
        let mut pixels = if self.onion && self.frame > 0 && !self.playing {
            self.doc
                .composite(self.frame - 1)
                .into_iter()
                .map(|p| [p[0], p[1], p[2], p[3] / 4])
                .collect()
        } else {
            vec![CLEAR; self.doc.width * self.doc.height]
        };
        for (dst, src) in pixels.iter_mut().zip(self.doc.composite(self.frame)) {
            *dst = mote::document::over(*dst, src, 255);
        }
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [self.doc.width, self.doc.height],
            &pixels.into_iter().flatten().collect::<Vec<_>>(),
        );
        let tex = self.texture.get_or_insert_with(|| {
            ctx.load_texture("canvas", image.clone(), egui::TextureOptions::NEAREST)
        });
        tex.set(image, egui::TextureOptions::NEAREST);
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(tex.id(), rect, uv, Color32::WHITE);
        if self.tiled {
            for y in -1..=1 {
                for x in -1..=1 {
                    if x != 0 || y != 0 {
                        painter.image(
                            tex.id(),
                            rect.translate(Vec2::new(x as f32 * size.x, y as f32 * size.y)),
                            uv,
                            Color32::from_white_alpha(100),
                        );
                    }
                }
            }
        }
        if self.grid && self.zoom >= 6.0 {
            for x in 0..=self.doc.width {
                let px = rect.left() + x as f32 * self.zoom;
                if px >= area.left() && px <= area.right() {
                    painter.line_segment(
                        [
                            Pos2::new(px, rect.top().max(area.top())),
                            Pos2::new(px, rect.bottom().min(area.bottom())),
                        ],
                        Stroke::new(0.5_f32, Color32::from_black_alpha(70)),
                    );
                }
            }
            for y in 0..=self.doc.height {
                let py = rect.top() + y as f32 * self.zoom;
                if py >= area.top() && py <= area.bottom() {
                    painter.line_segment(
                        [
                            Pos2::new(rect.left().max(area.left()), py),
                            Pos2::new(rect.right().min(area.right()), py),
                        ],
                        Stroke::new(0.5_f32, Color32::from_black_alpha(70)),
                    );
                }
            }
        }
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_gray(100)),
            egui::StrokeKind::Outside,
        );
        if let Some((l, t, r, b)) = self.bounds() {
            let selection = Rect::from_min_max(
                rect.min + Vec2::new(l as f32, t as f32) * self.zoom,
                rect.min + Vec2::new((r + 1) as f32, (b + 1) as f32) * self.zoom,
            );
            painter.rect_stroke(
                selection,
                0.0,
                Stroke::new(1.5_f32, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
        }
        let pointer = ctx.input(|i| i.pointer.interact_pos());
        let down = ctx.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
        let pressed = ctx.input(|i| i.pointer.primary_pressed() || i.pointer.secondary_pressed());
        if !self.playing
            && self.pending.is_none()
            && !self.new_dialog
            && self.resize.is_none()
            && !self.key_editor.open
            && !self.export_open
            && self.io_job.is_none()
            && let Some(pos) = pointer
        {
            let raw = (
                (pos.x - rect.left()) / self.zoom,
                (pos.y - rect.top()) / self.zoom,
            );
            let p = if matches!(self.tool, Tool::Pencil | Tool::Eraser) {
                // Keep the actual path outside the image. Pixel writes clip to
                // the document; clamping here would smear strokes along edges.
                (raw.0.floor() as i32, raw.1.floor() as i32)
            } else {
                (
                    (raw.0.floor() as i32).clamp(0, self.doc.width as i32 - 1),
                    (raw.1.floor() as i32).clamp(0, self.doc.height as i32 - 1),
                )
            };
            let inside = rect.contains(pos) && area.contains(pos) && response.hovered();
            if inside {
                let cursor = Rect::from_min_size(
                    rect.min + Vec2::new(p.0 as f32, p.1 as f32) * self.zoom,
                    Vec2::splat(self.zoom),
                );
                painter.rect_stroke(
                    cursor,
                    0.0,
                    Stroke::new(1.0_f32, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
            }
            let color = if self.tool == Tool::Eraser {
                CLEAR
            } else if ctx.input(|i| i.pointer.secondary_down()) {
                self.secondary
            } else {
                self.color
            };
            if pressed && inside {
                if self.tool == Tool::Picker || ctx.input(|i| i.modifiers.alt) {
                    self.color = self.doc.composite(self.frame)
                        [p.1 as usize * self.doc.width + p.0 as usize];
                } else if self.tool == Tool::Select {
                    self.start = Some(p);
                    self.selection = Some((p, p));
                } else if !self.doc.layers[self.layer].locked {
                    if self.tool == Tool::Fill {
                        if self.selection.is_some() {
                            self.status = "Deselect (Ctrl+D) before flood fill".into();
                        } else {
                            self.checkpoint();
                            self.doc.fill(self.frame, self.layer, p.0, p.1, color);
                        }
                    } else {
                        self.stroke_before = Some(self.doc.clone());
                        self.start = Some(p);
                        self.last = Some(p);
                        self.draw(p, p, color);
                    }
                }
            }
            if down && let Some(start) = self.start {
                if self.tool == Tool::Select {
                    self.selection = Some((start, p));
                } else if self.stroke_before.is_some() {
                    if matches!(
                        self.tool,
                        Tool::Line | Tool::Rectangle | Tool::Ellipse | Tool::Gradient
                    ) {
                        self.doc = self.stroke_before.as_ref().unwrap().clone();
                        self.draw(start, p, color);
                    } else {
                        self.draw(self.last.unwrap_or(p), p, color);
                    }
                    self.last = Some(p);
                }
            }
        }
        if !down {
            if let Some(before) = self.stroke_before.take()
                && before != self.doc
            {
                self.push_undo(before);
            }
            self.start = None;
            self.last = None;
        }
    }
    fn recovery_path() -> PathBuf {
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
            });
        base.join(format!("mote/recovery-{}.mote", std::process::id()))
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.io_job.is_some() {
            self.background_dialog(ctx);
            return;
        }
        self.keyboard_dialog(ctx);
        self.export_dialog(ctx);
        self.resize_dialog(ctx);
        if let Some(p) = self.pending {
            egui::Modal::new(egui::Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Keep your work?");
                ui.label(self.document_title(self.active_document));
                ui.label("This project has unsaved changes.");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() && self.save(false) {
                        self.pending = None;
                        self.perform(p, ctx);
                    }
                    if ui.button("Discard").clicked() {
                        self.pending = None;
                        self.perform(p, ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                        self.quit_reviewed.clear();
                    }
                });
            });
        }
        if self.new_dialog {
            egui::Modal::new(egui::Id::new("new")).show(ctx, |ui| {
                ui.heading("A little canvas. Endless possibilities.");
                ui.horizontal(|ui| {
                    ui.label("Width");
                    ui.add(egui::DragValue::new(&mut self.new_width).range(1..=2048));
                    ui.label("Height");
                    ui.add(egui::DragValue::new(&mut self.new_height).range(1..=2048));
                });
                ui.horizontal(|ui| {
                    for size in [16, 32, 64, 128] {
                        if ui.button(format!("{size} × {size}")).clicked() {
                            self.new_width = size;
                            self.new_height = size;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Create canvas").clicked() {
                        self.add_document(
                            Document::new(self.new_width, self.new_height),
                            None,
                            "Untitled".into(),
                        );
                        self.zoom = 0.0;
                        self.new_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        self.new_dialog = false;
                    }
                });
            });
        }
    }
}
impl eframe::App for Studio {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "workspace-v1", &self.dock);
        eframe::set_value(storage, "keybindings-v1", &self.bindings);
        eframe::set_value(storage, "export-options-v1", &self.export_options);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_background(ctx);
        self.theme.refresh(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending.is_none() && self.io_job.is_none() {
                self.quit_reviewed.clear();
                self.request(Pending::Close, ctx);
            }
        }
        self.shortcuts(ctx);
        if self.playing
            && self.tick.elapsed()
                >= Duration::from_millis(self.doc.frames[self.frame].duration as u64)
        {
            self.frame = (self.frame + 1) % self.doc.frames.len();
            self.tick = Instant::now();
        }
        self.toolbar(ctx);
        self.document_tabs(ctx);
        self.workspace(ctx);
        self.dialogs(ctx);
        if self.recovery_tick.elapsed() > Duration::from_secs(30)
            && self.stroke_before.is_none()
            && self.io_job.is_none()
        {
            self.save_recovery_documents();
            self.recovery_tick = Instant::now();
        }
        ctx.request_repaint_after(if self.io_job.is_some() {
            Duration::from_millis(50)
        } else if self.playing {
            Duration::from_millis(10)
        } else {
            Duration::from_secs(2)
        });
    }
}
fn rgba(p: Pixel) -> Color32 {
    Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])
}

fn main() -> eframe::Result {
    let mut app = Studio::default();
    if let Some(path) = std::env::args_os().nth(1) {
        let path = PathBuf::from(path);
        let result = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            Studio::import_png(&path)
        } else {
            Document::load(&path)
        };
        match result {
            Ok(doc) => {
                let native = path.extension().is_some_and(|e| e == "mote");
                app.replace(doc, if native { Some(path) } else { None });
                app.zoom = 0.0;
            }
            Err(e) => app.status = e,
        }
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Mote — Pixel Studio")
            .with_app_id("io.github.benryanx.mote")
            .with_inner_size([1360.0, 880.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "io.github.benryanx.mote",
        options,
        Box::new(|cc| {
            if let Some(storage) = cc.storage {
                if let Some(bindings) =
                    eframe::get_value::<Vec<keybindings::Binding>>(storage, "keybindings-v1")
                    && keybindings::valid(&bindings)
                {
                    app.bindings = bindings;
                }
                if let Some(options) = eframe::get_value(storage, "export-options-v1") {
                    app.export_options = options;
                }
            }
            if let Some(storage) = cc.storage
                && let Some(dock) = eframe::get_value(storage, "workspace-v1")
            {
                app.dock = dock;
            }
            match palettes::load() {
                Ok(library) => app.library = library,
                Err(error) => app.status = format!("Palette library: {error}"),
            }
            app.theme.refresh(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_actions_preserve_cels_timing_and_undo() {
        let mut app = Studio::default();
        app.doc.frames[0].duration = 200;
        app.doc.put(0, 0, 1, 1, [10, 20, 30, 128]);
        let original = app.doc.clone();
        app.frame_action(0, frames::Action::Duplicate);
        assert_eq!(app.doc.frames[0], app.doc.frames[1]);
        app.frame_action(1, frames::Action::Blank);
        assert!(app.doc.frames[2].cels[0].iter().all(|p| *p == CLEAR));
        app.frame_action(2, frames::Action::Left);
        assert!(app.doc.frames[1].cels[0].iter().all(|p| *p == CLEAR));
        app.frame_action(1, frames::Action::Delete);
        assert_eq!(app.doc.frames[0], app.doc.frames[1]);
        for _ in 0..4 {
            app.undo();
        }
        assert_eq!(app.doc, original);
        app.frame_action(0, frames::Action::Delete);
        assert_eq!(app.doc, original);
    }

    #[test]
    fn keyboard_undo_and_both_redo_bindings() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        app.checkpoint();
        app.doc.put(0, 0, 1, 1, [255; 4]);
        for (key, shift, dirty) in [
            (egui::Key::Z, false, false),
            (egui::Key::Y, false, true),
            (egui::Key::Z, false, false),
            (egui::Key::Z, true, true),
        ] {
            let modifiers = egui::Modifiers {
                ctrl: true,
                command: true,
                shift,
                ..Default::default()
            };
            let _ = ctx.run(
                egui::RawInput {
                    modifiers,
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    }],
                    ..Default::default()
                },
                |ctx| app.shortcuts(ctx),
            );
            assert_eq!(app.dirty(), dirty);
            let _ = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers,
                    }],
                    ..Default::default()
                },
                |_| {},
            );
        }
    }

    #[test]
    fn stroke_undo_redo_and_mirror() {
        let mut app = Studio {
            mirror_x: true,
            ..Default::default()
        };
        app.checkpoint();
        app.draw((2, 3), (9, 3), [255, 0, 0, 255]);
        assert_eq!(app.doc.frames[0].cels[0][3 * 32 + 2], [255, 0, 0, 255]);
        assert_eq!(app.doc.frames[0].cels[0][3 * 32 + 29], [255, 0, 0, 255]);
        app.undo();
        assert_eq!(app.doc, app.saved);
        app.redo();
        assert!(app.dirty());
    }

    #[test]
    fn selection_clips_strokes_and_paste_respects_lock() {
        let mut app = Studio {
            selection: Some(((2, 2), (4, 4))),
            ..Default::default()
        };
        app.draw((0, 3), (9, 3), [255; 4]);
        assert_eq!(app.doc.frames[0].cels[0][3 * 32 + 1], CLEAR);
        assert_eq!(app.doc.frames[0].cels[0][3 * 32 + 3], [255; 4]);
        app.copy();
        app.selection = Some(((8, 8), (10, 10)));
        app.doc.layers[0].locked = true;
        let before = app.doc.clone();
        app.paste();
        assert_eq!(before, app.doc);
        app.doc.layers[0].locked = false;
        app.paste();
        assert_eq!(app.doc.frames[0].cels[0][9 * 32 + 9], [255; 4]);
    }

    #[test]
    fn freehand_stroke_leaves_and_reenters_without_smearing_edge() {
        for tool in [Tool::Pencil, Tool::Eraser] {
            let ctx = egui::Context::default();
            let mut app = Studio {
                tool,
                zoom: 10.0,
                ..Default::default()
            };
            app.doc = Document::new(8, 8);
            if tool == Tool::Eraser {
                app.doc.frames[0].cels[0].fill([255; 4]);
            }
            app.saved = app.doc.clone();
            let original = app.doc.clone();
            let mut center = Pos2::ZERO;
            let mut render = |app: &mut Studio, events| {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(400.0))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            center = ui.available_rect_before_wrap().center();
                            app.canvas_ui(ui);
                        });
                    },
                );
                center
            };
            let center = render(&mut app, vec![]);
            let point = |x: f32, y: f32| center + Vec2::new((x - 3.5) * 10.0, (y - 3.5) * 10.0);
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            let start = point(6.0, 1.0);
            render(
                &mut app,
                vec![egui::Event::PointerMoved(start), button(start, true)],
            );
            for (x, y) in [(11.0, 1.0), (11.0, 6.0), (6.0, 6.0)] {
                render(&mut app, vec![egui::Event::PointerMoved(point(x, y))]);
            }
            render(&mut app, vec![button(point(6.0, 6.0), false)]);
            let pixels = &app.doc.frames[0].cels[0];
            assert_eq!(
                pixels[3 * 8 + 7],
                original.frames[0].cels[0][3 * 8 + 7],
                "Outside movement must not paint the edge"
            );
            assert_ne!(
                pixels[6 * 8 + 7],
                original.frames[0].cels[0][6 * 8 + 7],
                "Re-entry must continue painting"
            );
            assert_eq!(app.undo.len(), 1);
            app.undo();
            assert_eq!(app.doc, original);
            app.redo();
            assert!(app.dirty());
        }
    }
    #[test]
    fn interface_and_pointer_stroke_render_without_display() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        let render = |app: &mut Studio, events: Vec<egui::Event>| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360.0, 880.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    app.toolbar(ctx);
                    app.workspace(ctx);
                    app.dialogs(ctx);
                },
            )
        };
        assert!(!render(&mut app, vec![]).shapes.is_empty());
        let pos = Pos2::new(650.0, 400.0);
        render(
            &mut app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        render(
            &mut app,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(app.dirty(), "Canvas click must paint a pixel");
        assert_eq!(app.undo.len(), 1, "One gesture produces one undo entry");
        app.undo();
        assert_eq!(app.doc, app.saved);
    }
}
