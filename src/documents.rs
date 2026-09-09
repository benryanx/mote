use crate::{Pending, Studio};
use eframe::egui::{self, Vec2};
use mote::document::{Document, Pixel};
use std::path::PathBuf;

pub struct Session {
    pub(crate) document_id: u64,
    pub doc: Document,
    pub saved: Document,
    pub path: Option<PathBuf>,
    pub title: String,
    undo: Vec<Document>,
    redo: Vec<Document>,
    frame: usize,
    layer: usize,
    zoom: f32,
    pan: Vec2,
    selection: Option<((i32, i32), (i32, i32))>,
    color: Pixel,
    secondary: Pixel,
    palette_name: String,
}
impl Session {
    pub fn blank() -> Self {
        let doc = Document::new(1, 1);
        Self {
            document_id: 0,
            saved: doc.clone(),
            doc,
            path: None,
            title: String::new(),
            undo: vec![],
            redo: vec![],
            frame: 0,
            layer: 0,
            zoom: 0.0,
            pan: Vec2::ZERO,
            selection: None,
            color: [0; 4],
            secondary: [0; 4],
            palette_name: String::new(),
        }
    }
    fn swap(&mut self, app: &mut Studio) {
        macro_rules! swap {($($field:ident),*)=>{$(std::mem::swap(&mut self.$field,&mut app.$field);)*}}
        swap!(
            document_id,
            doc,
            saved,
            path,
            title,
            undo,
            redo,
            frame,
            layer,
            zoom,
            pan,
            selection,
            color,
            secondary,
            palette_name
        );
    }
}
impl Studio {
    pub fn receive_document(&mut self, path: PathBuf, doc: Document) {
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        if let Some(index) = (0..self.documents.len()).find(|i| {
            let existing = if *i == self.active_document {
                self.path.as_ref()
            } else {
                self.documents[*i].as_ref().unwrap().path.as_ref()
            };
            existing.is_some_and(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()) == path)
        }) {
            self.switch_document(index);
            return;
        }
        let png = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"));
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.add_document(doc, if png { None } else { Some(path) }, title);
        self.status = "Opened in a new tab".into();
    }
    pub fn filename(&self, extension: &str) -> String {
        let name = self
            .path
            .as_deref()
            .unwrap_or_else(|| std::path::Path::new(&self.title));
        format!(
            "{}.{}",
            name.file_stem().unwrap_or_default().to_string_lossy(),
            extension
        )
    }
    #[cfg(test)]
    pub fn open_path(&mut self, path: PathBuf) {
        let result = if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            Self::import_png(&path)
        } else {
            Document::load(&path)
        };
        self.finish_background(
            Ok(crate::background::Outcome::Opened(vec![(path, result)])),
            &egui::Context::default(),
        );
    }
    pub fn finish_gesture(&mut self) {
        if let Some(before) = self.stroke_before.take()
            && before != self.doc
        {
            self.push_undo(before);
        }
        self.start = None;
        self.last = None;
    }
    pub fn switch_document(&mut self, index: usize) {
        if index == self.active_document || index >= self.documents.len() {
            return;
        }
        self.finish_gesture();
        let mut incoming = self.documents[index].take().expect("inactive document");
        incoming.swap(self);
        self.documents[self.active_document] = Some(incoming);
        self.active_document = index;
        self.playing = false;
        self.resize = None;
    }
    pub fn add_document(&mut self, doc: Document, path: Option<PathBuf>, title: String) {
        self.finish_gesture();
        let colors = (self.color, self.secondary);
        let mut previous = Session::blank();
        previous.swap(self);
        self.documents[self.active_document] = Some(previous);
        self.active_document = self.documents.len();
        self.documents.push(None);
        self.replace(doc, path);
        self.document_id = self.next_document_id;
        self.next_document_id += 1;
        self.title = title;
        self.zoom = 0.0;
        // Keep drawing colors when opening another image.
        self.color = colors.0;
        self.secondary = colors.1;
        self.palette_name = "My palette".into();
    }
    pub fn document_title(&self, index: usize) -> String {
        let (path, title, dirty) = if index == self.active_document {
            (&self.path, &self.title, self.dirty())
        } else {
            let s = self.documents[index].as_ref().unwrap();
            (&s.path, &s.title, s.doc != s.saved)
        };
        let name = path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| title.clone());
        format!("{name}{}", if dirty { " •" } else { "" })
    }
    pub fn close_active_document(&mut self) {
        self.finish_gesture();
        if self.documents.len() == 1 {
            self.replace(Document::new(32, 32), None);
            self.title = "Untitled".into();
            self.document_id = self.next_document_id;
            self.next_document_id += 1;
            return;
        }
        self.documents.remove(self.active_document);
        self.active_document = self.active_document.min(self.documents.len() - 1);
        let mut next = self.documents[self.active_document].take().unwrap();
        next.swap(self);
        self.playing = false;
        self.resize = None;
    }
    pub fn next_unsaved_document(&self) -> Option<usize> {
        (0..self.documents.len()).find(|i| {
            !self.quit_reviewed.contains(i)
                && if *i == self.active_document {
                    self.dirty()
                } else {
                    let s = self.documents[*i].as_ref().unwrap();
                    s.doc != s.saved
                }
        })
    }
    pub fn document_tabs(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("documents").show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    let enabled = self.pending.is_none()
                        && !self.new_dialog
                        && self.resize.is_none()
                        && !self.key_editor.open
                        && !self.export_open
                        && self.io_job.is_none();
                    let mut switch = None;
                    let mut close = None;
                    for i in 0..self.documents.len() {
                        ui.push_id(i, |ui| {
                            ui.add_enabled_ui(enabled, |ui| {
                                if ui
                                    .selectable_label(
                                        i == self.active_document,
                                        self.document_title(i),
                                    )
                                    .clicked()
                                {
                                    switch = Some(i);
                                }
                                if ui
                                    .small_button("×")
                                    .on_hover_text(self.command_label(
                                        crate::keybindings::Command::Close,
                                        "Close document",
                                    ))
                                    .clicked()
                                {
                                    close = Some(i);
                                }
                            });
                        });
                    }
                    if let Some(i) = switch {
                        self.switch_document(i);
                    }
                    if let Some(i) = close {
                        self.switch_document(i);
                        self.request(Pending::CloseTab, ctx);
                    }
                });
            });
        });
    }
    pub fn save_recovery_documents(&mut self) {
        let base = Self::recovery_path();
        let parent = base.parent().unwrap();
        let result = (|| -> Result<(), String> {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            for i in 0..self.documents.len() {
                let (doc, dirty, id) = if i == self.active_document {
                    (&self.doc, self.dirty(), self.document_id)
                } else {
                    let s = self.documents[i].as_ref().unwrap();
                    (&s.doc, s.doc != s.saved, s.document_id)
                };
                if dirty {
                    doc.save(&parent.join(format!(
                        "recovery-{}-tab-{}.mote",
                        std::process::id(),
                        id
                    )))?;
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.status = format!("Recovery save failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tabs_preserve_history_view_selection_and_save_only_active() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.mote");
        let b = dir.path().join("b.mote");
        let mut app = Studio {
            path: Some(a.clone()),
            ..Default::default()
        };
        app.checkpoint();
        app.doc.put(0, 0, 2, 2, [255; 4]);
        app.zoom = 7.0;
        app.selection = Some(((1, 1), (3, 3)));
        let first = app.doc.clone();
        app.add_document(Document::new(8, 8), Some(b.clone()), "b.mote".into());
        app.checkpoint();
        app.doc.put(0, 0, 1, 1, [17; 4]);
        assert!(!app.save(false));
        app.wait_background_for_test(&egui::Context::default());
        assert!(!a.exists());
        let export = dir.path().join("active.png");
        app.export_to(0, &export).unwrap();
        let image = image::open(export).unwrap().into_rgba8();
        assert_eq!(image.dimensions(), (8, 8));
        assert_eq!(image.get_pixel(1, 1).0, [17; 4]);
        assert_eq!(Document::load(&b).unwrap(), app.doc);
        app.switch_document(0);
        assert_eq!(app.doc, first);
        assert_eq!(app.zoom, 7.0);
        assert!(app.selection.is_some());
        assert!(app.dirty());
        app.undo();
        assert!(!app.dirty());
        app.redo();
        assert_eq!(app.doc, first);
        app.switch_document(1);
        assert!(!app.dirty());
        assert_eq!(app.doc.width, 8);
        app.undo();
        assert_eq!(app.doc.frames[0].cels[0][9], [0; 4]);
        assert_eq!(Document::load(&b).unwrap().frames[0].cels[0][9], [17; 4]);
    }
    #[test]
    fn new_open_does_not_discard_dirty_work_and_duplicate_open_focuses() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loaded.mote");
        Document::new(6, 6).save(&path).unwrap();
        let mut app = Studio::default();
        app.checkpoint();
        app.doc.put(0, 0, 0, 0, [255; 4]);
        let before = app.doc.clone();
        app.open_path(path.clone());
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.doc.width, 6);
        app.switch_document(0);
        assert_eq!(app.doc, before);
        app.open_path(path);
        assert_eq!(app.documents.len(), 2);
        assert_eq!(app.active_document, 1);
        app.open_path(dir.path().join("missing.mote"));
        assert_eq!(app.documents.len(), 2);
    }
    #[test]
    fn close_and_quit_prompt_for_each_dirty_document() {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        app.doc.put(0, 0, 0, 0, [255; 4]);
        app.add_document(Document::new(8, 8), None, "Second".into());
        app.doc.put(0, 0, 0, 0, [17; 4]);
        app.request(Pending::Close, &ctx);
        assert_eq!(app.active_document, 0);
        assert!(app.pending.is_some());
        assert!(!app.allow_close);
        app.pending = None;
        app.perform(Pending::Close, &ctx);
        assert_eq!(app.active_document, 1);
        assert!(!app.allow_close);
        // Cancel leaves both documents intact.
        app.pending = None;
        app.quit_reviewed.clear();
        assert_eq!(app.documents.len(), 2);
        app.request(Pending::CloseTab, &ctx);
        assert!(app.pending.is_some());
        assert_eq!(app.documents.len(), 2);
        app.pending = None;
        app.perform(Pending::CloseTab, &ctx);
        assert_eq!(app.documents.len(), 1);
        assert_eq!(app.doc.width, 32);
        assert!(app.dirty());
    }
}
