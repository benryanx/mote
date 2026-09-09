use crate::{Pending, Studio, Tool, frames, resize, workspace};
use eframe::egui::{self, Key};
use serde::{Deserialize, Serialize};

macro_rules! commands {($( $id:ident => ($label:literal,$keys:literal)),* $(,)?)=>{
    #[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
    pub enum Command {$($id),*}
    pub const COMMANDS:&[(Command,&str,&str)]=&[$((Command::$id,$label,$keys)),*];
}}
commands! {
    New=> ("File / New document","Ctrl+N"),Open=>("File / Open images","Ctrl+O"),
    Save=>("File / Save","Ctrl+S"),SaveAs=>("File / Save as","Ctrl+Shift+S"),Export=>("File / Export as","Ctrl+Alt+Shift+S"),
    Close=>("File / Close document","Ctrl+W"),NextDoc=>("File / Next document","Ctrl+Tab"),PrevDoc=>("File / Previous document","Ctrl+Shift+Tab"),
    CanvasSize=>("File / Canvas size",""),SpriteSize=>("File / Sprite size",""),
    Undo=>("Edit / Undo","Ctrl+Z"),Redo=>("Edit / Redo","Ctrl+Shift+Z|Ctrl+Y"),Copy=>("Edit / Copy","Ctrl+C"),Paste=>("Edit / Paste","Ctrl+V"),
    Clear=>("Edit / Clear selection","Delete"),Deselect=>("Edit / Deselect","Ctrl+D|Escape"),FlipH=>("Edit / Flip horizontal",""),FlipV=>("Edit / Flip vertical",""),
    Shortcuts=>("Edit / Keyboard shortcuts","Ctrl+Alt+Shift+K"),
    NewLayer=>("Layer / New layer","Shift+N"),DuplicateLayer=>("Layer / Duplicate layer","Ctrl+Shift+J"),DeleteLayer=>("Layer / Delete layer",""),
    RaiseLayer=>("Layer / Raise",""),LowerLayer=>("Layer / Lower",""),LayerVisible=>("Layer / Toggle visibility",""),LayerLock=>("Layer / Toggle lock",""),
    NewFrame=>("Frame / Duplicate frame","Alt+N"),BlankFrame=>("Frame / New blank frame","Alt+Shift+N"),DeleteFrame=>("Frame / Delete frame",""),
    PrevFrame=>("Frame / Previous","ArrowLeft"),NextFrame=>("Frame / Next","ArrowRight"),EarlierFrame=>("Frame / Move earlier","Alt+ArrowLeft"),LaterFrame=>("Frame / Move later","Alt+ArrowRight"),
    Play=>("Frame / Play or pause","Space"),Onion=>("View / Onion skin",""),Grid=>("View / Pixel grid","Ctrl+Quote"),Tile=>("View / Tile preview",""),
    Fit=>("View / Fit canvas","F"),ZoomIn=>("View / Zoom in","Plus"),ZoomOut=>("View / Zoom out","Minus"),Center=>("View / Center canvas",""),ResetLayout=>("View / Reset workspace",""),
    Pencil=>("Tool / Pencil","B"),Eraser=>("Tool / Eraser","E"),Fill=>("Tool / Fill","G"),Picker=>("Tool / Eyedropper","I"),
    Line=>("Tool / Line","L"),Rectangle=>("Tool / Rectangle","U"),Ellipse=>("Tool / Ellipse","O"),Select=>("Tool / Selection","M"),Gradient=>("Tool / Gradient","D|Shift+G"),
    Swap=>("Color / Swap foreground and background","X"),BrushSmaller=>("Tool / Smaller brush","OpenBracket"),BrushLarger=>("Tool / Larger brush","CloseBracket"),
    MirrorX=>("Tool / Mirror X",""),MirrorY=>("Tool / Mirror Y",""),Filled=>("Tool / Filled shapes",""),Help=>("Help / Keyboard guide","F1")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    key: Key,
    ctrl: bool,
    alt: bool,
    shift: bool,
}
impl Chord {
    fn parse(text: &str) -> Self {
        let parts: Vec<_> = text.split('+').collect();
        Self {
            key: Key::from_name(parts.last().unwrap()).expect("default key"),
            ctrl: parts.contains(&"Ctrl"),
            alt: parts.contains(&"Alt"),
            shift: parts.contains(&"Shift"),
        }
    }
    fn from_event(key: Key, m: egui::Modifiers) -> Self {
        Self {
            key,
            ctrl: m.ctrl || m.command,
            alt: m.alt,
            shift: m.shift,
        }
    }
    pub fn label(self) -> String {
        format!(
            "{}{}{}{}",
            if self.ctrl { "Ctrl+" } else { "" },
            if self.alt { "Alt+" } else { "" },
            if self.shift { "Shift+" } else { "" },
            self.key.name()
        )
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Binding {
    pub command: Command,
    pub keys: Vec<Chord>,
}
pub fn defaults() -> Vec<Binding> {
    COMMANDS
        .iter()
        .map(|(command, _, keys)| Binding {
            command: *command,
            keys: keys
                .split('|')
                .filter(|s| !s.is_empty())
                .map(Chord::parse)
                .collect(),
        })
        .collect()
}
pub fn valid(bindings: &[Binding]) -> bool {
    if bindings.len() != COMMANDS.len() {
        return false;
    }
    if bindings.iter().zip(COMMANDS).any(|(b, c)| b.command != c.0) {
        return false;
    }
    let mut seen = vec![];
    for (command, _, _) in COMMANDS {
        let b: Vec<_> = bindings.iter().filter(|b| b.command == *command).collect();
        if b.len() != 1 || b[0].keys.len() > 2 {
            return false;
        }
        for key in &b[0].keys {
            if seen.contains(key) {
                return false;
            }
            seen.push(*key);
        }
    }
    true
}
pub fn tool_command(tool: Tool) -> Command {
    match tool {
        Tool::Pencil => Command::Pencil,
        Tool::Eraser => Command::Eraser,
        Tool::Fill => Command::Fill,
        Tool::Picker => Command::Picker,
        Tool::Line => Command::Line,
        Tool::Rectangle => Command::Rectangle,
        Tool::Ellipse => Command::Ellipse,
        Tool::Select => Command::Select,
        Tool::Gradient => Command::Gradient,
    }
}
#[derive(Default)]
pub struct Editor {
    pub open: bool,
    search: String,
    pub(crate) capture: Option<(usize, usize)>,
    pub(crate) message: String,
}
impl Studio {
    fn shortcut_profiles(&mut self, ui: &mut egui::Ui) {
        if ui.button("Import…").clicked() {
            self.start_background("Importing shortcuts", || {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("Mote shortcuts", &["json"])
                    .pick_file()
                else {
                    return Ok(crate::background::Outcome::Canceled);
                };
                use std::io::Read;
                let mut bytes = vec![];
                std::fs::File::open(path)
                    .map_err(|e| e.to_string())?
                    .take(131073)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                if bytes.len() > 131072 {
                    return Err("Shortcut file exceeds 128 KB".into());
                }
                let bindings =
                    serde_json::from_slice::<Vec<Binding>>(&bytes).map_err(|e| e.to_string())?;
                if !valid(&bindings) {
                    return Err("Invalid or conflicting shortcut profile".into());
                }
                Ok(crate::background::Outcome::Bindings(bindings))
            });
        }
        if ui.button("Export…").clicked() {
            let bindings = self.bindings.clone();
            self.start_background("Exporting shortcuts", move || {
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("Mote shortcuts", &["json"])
                    .set_file_name("mote-shortcuts.json")
                    .save_file()
                else {
                    return Ok(crate::background::Outcome::Canceled);
                };
                let mut file = tempfile::NamedTempFile::new_in(
                    path.parent().unwrap_or(std::path::Path::new(".")),
                )
                .map_err(|e| e.to_string())?;
                serde_json::to_writer_pretty(file.as_file_mut(), &bindings)
                    .map_err(|e| e.to_string())?;
                file.as_file().sync_all().map_err(|e| e.to_string())?;
                file.persist(path).map_err(|e| e.to_string())?;
                Ok(crate::background::Outcome::ProfileSaved)
            });
        }
    }
    pub fn binding_label(&self, c: Command) -> String {
        self.bindings
            .iter()
            .find(|b| b.command == c)
            .map(|b| {
                b.keys
                    .iter()
                    .map(|k| k.label())
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .unwrap_or_default()
    }
    pub fn command_label(&self, c: Command, name: &str) -> String {
        let key = self.binding_label(c);
        if key.is_empty() {
            name.into()
        } else {
            format!("{name}    {key}")
        }
    }
    pub fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input()
            || self.pending.is_some()
            || self.new_dialog
            || self.resize.is_some()
            || self.key_editor.open
            || self.export_open
            || self.stroke_before.is_some()
            || self.io_job.is_some()
        {
            return;
        }
        let command = ctx.input(|i| {
            i.events.iter().find_map(|event| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                {
                    let chord = Chord::from_event(*key, *modifiers);
                    self.bindings
                        .iter()
                        .find(|b| b.keys.contains(&chord))
                        .map(|b| b.command)
                } else {
                    None
                }
            })
        });
        if let Some(c) = command {
            self.run_command(c, ctx);
        }
    }
    pub fn run_command(&mut self, c: Command, ctx: &egui::Context) {
        use Command::*;
        match c {
            New => self.request(Pending::New, ctx),
            Open => self.open(),
            Save => {
                self.save(false);
            }
            SaveAs => {
                self.save(true);
            }
            Export => self.open_export(),
            Close => self.request(Pending::CloseTab, ctx),
            NextDoc | PrevDoc => {
                let n = self.documents.len();
                self.switch_document(
                    (self.active_document + if c == NextDoc { 1 } else { n - 1 }) % n,
                );
            }
            CanvasSize => self.open_resize(resize::Kind::Canvas),
            SpriteSize => self.open_resize(resize::Kind::Sprite),
            Undo => self.undo(),
            Redo => self.redo(),
            Copy => self.copy(),
            Paste => self.paste(),
            Clear => self.delete_selection(),
            Deselect => self.selection = None,
            FlipH => self.flip(true),
            FlipV => self.flip(false),
            Shortcuts | Help => {
                self.key_editor.open = true;
                self.playing = false;
            }
            NewLayer | DuplicateLayer => self.add_layer(c == DuplicateLayer),
            DeleteLayer => self.delete_layer(),
            RaiseLayer | LowerLayer => self.move_layer(c == RaiseLayer),
            LayerVisible => {
                self.checkpoint();
                self.doc.layers[self.layer].visible = !self.doc.layers[self.layer].visible;
            }
            LayerLock => {
                self.checkpoint();
                self.doc.layers[self.layer].locked = !self.doc.layers[self.layer].locked;
            }
            NewFrame => self.frame_action(self.frame, frames::Action::Duplicate),
            BlankFrame => self.frame_action(self.frame, frames::Action::Blank),
            DeleteFrame => self.frame_action(self.frame, frames::Action::Delete),
            EarlierFrame => self.frame_action(self.frame, frames::Action::Left),
            LaterFrame => self.frame_action(self.frame, frames::Action::Right),
            PrevFrame => {
                self.frame = self.frame.saturating_sub(1);
                self.playing = false;
            }
            NextFrame => {
                self.frame = (self.frame + 1).min(self.doc.frames.len() - 1);
                self.playing = false;
            }
            Play => {
                self.playing = !self.playing;
                self.tick = std::time::Instant::now();
            }
            Onion => self.onion = !self.onion,
            Grid => self.grid = !self.grid,
            Tile => self.tiled = !self.tiled,
            Fit => {
                self.zoom = 0.0;
                self.pan = egui::Vec2::ZERO;
            }
            Center => self.pan = egui::Vec2::ZERO,
            ZoomIn => self.zoom = (self.zoom.max(1.0) * 1.25).min(64.0),
            ZoomOut => self.zoom = (self.zoom.max(1.0) / 1.25).max(0.25),
            ResetLayout => self.dock = workspace::default_layout(),
            Pencil => self.tool = Tool::Pencil,
            Eraser => self.tool = Tool::Eraser,
            Fill => self.tool = Tool::Fill,
            Picker => self.tool = Tool::Picker,
            Line => self.tool = Tool::Line,
            Rectangle => self.tool = Tool::Rectangle,
            Ellipse => self.tool = Tool::Ellipse,
            Select => self.tool = Tool::Select,
            Gradient => self.tool = Tool::Gradient,
            Swap => std::mem::swap(&mut self.color, &mut self.secondary),
            BrushSmaller => self.brush = (self.brush - 1).max(1),
            BrushLarger => self.brush = (self.brush + 1).min(16),
            MirrorX => self.mirror_x = !self.mirror_x,
            MirrorY => self.mirror_y = !self.mirror_y,
            Filled => self.filled = !self.filled,
        }
    }
    pub fn add_layer(&mut self, duplicate: bool) {
        if !self
            .doc
            .can_grow(self.doc.layers.len() + 1, self.doc.frames.len())
        {
            self.status = "Project layer limit reached".into();
            return;
        }
        self.checkpoint();
        let index = self.layer + 1;
        let layer = if duplicate {
            let mut l = self.doc.layers[self.layer].clone();
            l.name = format!("{} copy", l.name);
            l
        } else {
            mote::document::Layer {
                name: format!("Layer {}", self.doc.layers.len() + 1),
                visible: true,
                locked: false,
                opacity: 255,
            }
        };
        self.doc.layers.insert(index, layer);
        for f in &mut self.doc.frames {
            let cel = if duplicate {
                f.cels[self.layer].clone()
            } else {
                vec![mote::document::CLEAR; self.doc.width * self.doc.height]
            };
            f.cels.insert(index, cel);
        }
        self.layer = index;
    }
    pub fn delete_layer(&mut self) {
        if self.doc.layers.len() > 1 {
            self.checkpoint();
            self.doc.layers.remove(self.layer);
            for f in &mut self.doc.frames {
                f.cels.remove(self.layer);
            }
            self.normalize();
        }
    }
    pub fn move_layer(&mut self, up: bool) {
        let target = if up {
            self.layer + 1
        } else {
            self.layer.saturating_sub(1)
        };
        if target != self.layer && target < self.doc.layers.len() {
            self.checkpoint();
            self.doc.layers.swap(self.layer, target);
            for f in &mut self.doc.frames {
                f.cels.swap(self.layer, target);
            }
            self.layer = target;
        }
    }
    pub fn keyboard_dialog(&mut self, ctx: &egui::Context) {
        if !self.key_editor.open {
            return;
        }
        if let Some((index, slot)) = self.key_editor.capture {
            let key = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Key {
                        key,
                        modifiers,
                        pressed: true,
                        repeat: false,
                        ..
                    } = e
                    {
                        Some(Chord::from_event(*key, *modifiers))
                    } else {
                        None
                    }
                })
            });
            if let Some(key) = key {
                if let Some(other) = self
                    .bindings
                    .iter()
                    .enumerate()
                    .find(|(i, b)| *i != index && b.keys.contains(&key))
                    .map(|(i, _)| i)
                {
                    self.key_editor.message = format!(
                        "{} is assigned to {}. Clear that binding first.",
                        key.label(),
                        COMMANDS[other].1
                    );
                } else {
                    let keys = &mut self.bindings[index].keys;
                    if keys.contains(&key) {
                        self.key_editor.message = "Already assigned to this command.".into();
                    } else {
                        if slot < keys.len() {
                            keys[slot] = key;
                        } else {
                            keys.push(key);
                        }
                        self.key_editor.message = "Shortcut updated".into();
                    }
                    self.key_editor.capture = None;
                }
            }
        }
        egui::Modal::new(egui::Id::new("keyboard_settings")).show(ctx,|ui|{
            ui.set_width(600.0_f32.min(ctx.content_rect().width()-48.0));
            ui.heading("Keyboard Shortcuts");
            ui.label("Click a binding, then press your key combination. Up to two per command.");
            ui.add_enabled(self.key_editor.capture.is_none(),egui::TextEdit::singleline(&mut self.key_editor.search).hint_text("Search tools, layers, frames, export…"));
            if self.key_editor.capture.is_some(){ui.horizontal(|ui|{ui.label("Listening…");if ui.button("Cancel capture").clicked(){self.key_editor.capture=None;}});}
            ui.label(&self.key_editor.message);
            egui::ScrollArea::vertical().max_height((ctx.content_rect().height()-250.0).max(100.0)).show(ui,|ui|{
                for(index,(_,name,_))in COMMANDS.iter().enumerate(){if !name.to_lowercase().contains(&self.key_editor.search.to_lowercase()){continue;}
                    ui.push_id(index,|ui|{ui.horizontal_wrapped(|ui|{
                        ui.label(*name);
                        for slot in 0..2{
                            let label=self.bindings[index].keys.get(slot).map(|k|k.label()).unwrap_or("Assign…".into());
                            if ui.button(label).clicked(){self.key_editor.capture=Some((index,slot));self.key_editor.message.clear();}
                        }
                        if ui.small_button("Clear").clicked(){self.bindings[index].keys.clear();self.key_editor.capture=None;}
                    });});
                }
            });
            ui.separator();
            ui.horizontal(|ui|{
                self.shortcut_profiles(ui);
                if ui.button("Restore defaults").clicked(){self.bindings=defaults();self.key_editor.capture=None;}
                if ui.button("Done").clicked(){self.key_editor.open=false;self.key_editor.capture=None;}
            });
            ui.small("Changes apply immediately and are saved on exit. Text fields keep their normal editing shortcuts.");
        });
    }
}

#[cfg(test)]
#[path = "keybindings_tests.rs"]
mod tests;
