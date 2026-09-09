use crate::{Studio, keybindings};
use eframe::egui;
use mote::document::Document;
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};

pub enum Outcome {
    Saved {
        document_id: u64,
        doc: Document,
        path: PathBuf,
    },
    Opened(Vec<(PathBuf, Result<Document, String>)>),
    Exported(PathBuf),
    Bindings(Vec<keybindings::Binding>),
    ProfileSaved,
    Canceled,
}

pub struct Job {
    pub receiver: Receiver<Result<Outcome, String>>,
    label: String,
}
impl Studio {
    pub fn start_background(
        &mut self,
        label: &str,
        work: impl FnOnce() -> Result<Outcome, String> + Send + 'static,
    ) {
        if self.io_job.is_some() {
            return;
        }
        self.finish_gesture();
        self.playing = false;
        let (sender, receiver) = mpsc::channel();
        match std::thread::Builder::new()
            .name("mote-file-io".into())
            .spawn(move || {
                let _ = sender.send(work());
            }) {
            Ok(_) => {
                self.io_job = Some(Job {
                    receiver,
                    label: label.into(),
                })
            }
            Err(e) => self.status = format!("Could not start file operation: {e}"),
        }
    }
    pub fn poll_background(&mut self, ctx: &egui::Context) {
        let Some(job) = &self.io_job else {
            return;
        };
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                Err("File operation stopped unexpectedly; your project remains open.".into())
            }
        };
        self.io_job = None;
        self.finish_background(result, ctx);
    }
    pub fn finish_background(&mut self, result: Result<Outcome, String>, ctx: &egui::Context) {
        match result {
            Ok(Outcome::Saved {
                document_id,
                doc,
                path,
            }) => {
                if self.document_id == document_id {
                    self.saved = doc;
                    self.path = Some(path);
                    self.status = "Project saved".into();
                    if !self.dirty()
                        && let Some(p) = self.pending.take()
                    {
                        self.perform(p, ctx);
                    }
                } else if let Some(session) = self
                    .documents
                    .iter_mut()
                    .flatten()
                    .find(|s| s.document_id == document_id)
                {
                    session.saved = doc;
                    session.path = Some(path);
                }
            }
            Ok(Outcome::Opened(files)) => {
                for (path, result) in files {
                    match result {
                        Ok(doc) => self.receive_document(path, doc),
                        Err(e) => self.status = e,
                    }
                }
            }
            Ok(Outcome::Exported(path)) => {
                self.status = format!("Exported {}", path.display());
                self.export_open = false;
            }
            Ok(Outcome::Bindings(bindings)) => {
                self.bindings = bindings;
                self.key_editor.message = "Imported shortcut profile".into();
                self.key_editor.capture = None;
            }
            Ok(Outcome::ProfileSaved) => {
                self.key_editor.message = "Exported shortcut profile".into()
            }
            Ok(Outcome::Canceled) => self.status = "File dialog canceled".into(),
            Err(e) => {
                self.status = e.clone();
                if self.key_editor.open {
                    self.key_editor.message = e;
                }
            }
        }
    }
    pub fn background_dialog(&self, ctx: &egui::Context) {
        if let Some(job) = &self.io_job {
            egui::Modal::new(egui::Id::new("file_operation")).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.heading(&job.label);
                });
                ui.label("Complete or cancel the file chooser to return to the editor.");
            });
        }
    }
    #[cfg(test)]
    pub fn wait_background_for_test(&mut self, ctx: &egui::Context) {
        let result = self
            .io_job
            .take()
            .unwrap()
            .receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        self.finish_background(result, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pending;

    #[test]
    fn waiting_worker_keeps_ui_polling_and_rendering() {
        let mut app = Studio::default();
        let ctx = egui::Context::default();
        let (release, gate) = mpsc::channel();
        app.start_background("Test chooser", move || {
            gate.recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            Ok(Outcome::Canceled)
        });
        for _ in 0..3 {
            app.poll_background(&ctx);
            assert!(app.io_job.is_some());
            let _ = ctx.run(Default::default(), |ctx| app.dialogs(ctx));
        }
        release.send(()).unwrap();
        app.wait_background_for_test(&ctx);
        assert!(app.io_job.is_none());
    }

    #[test]
    fn cancel_and_failure_preserve_dirty_document_and_pending_close() {
        let mut app = Studio::default();
        app.doc.put(0, 0, 0, 0, [255; 4]);
        let before = app.doc.clone();
        app.pending = Some(Pending::CloseTab);
        let ctx = egui::Context::default();
        for result in [Ok(Outcome::Canceled), Err("Save failed".into())] {
            app.finish_background(result, &ctx);
            assert_eq!(app.doc, before);
            assert!(app.dirty());
            assert!(matches!(app.pending, Some(Pending::CloseTab)));
            assert!(app.path.is_none());
        }
    }

    #[test]
    fn successful_save_completes_pending_tab_close() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = Studio::default();
        app.add_document(
            Document::new(8, 8),
            Some(dir.path().join("saved.mote")),
            "second".into(),
        );
        app.doc.put(0, 0, 0, 0, [255; 4]);
        let expected = app.doc.clone();
        app.pending = Some(Pending::CloseTab);
        app.save(false);
        app.wait_background_for_test(&egui::Context::default());
        assert_eq!(
            Document::load(&dir.path().join("saved.mote")).unwrap(),
            expected
        );
        assert_eq!(app.documents.len(), 1);
        assert!(app.pending.is_none());
    }
}
