//! Prompt-to-art protocol. Model output is data, never executable code.
use crate::Studio;
use eframe::egui;
use mote::document::{CLEAR, Document, Layer};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

const MAX_RESPONSE: u64 = 2 * 1024 * 1024;

#[cfg(test)]
#[path = "ai_tests.rs"]
mod tests;

#[derive(Clone, Copy, PartialEq)]
pub enum Provider {
    Agent,
    Ollama,
    Compatible,
}

pub struct Assistant {
    pub open: bool,
    settings: Option<ConnectionSettings>,
    provider: Provider,
    endpoint: String,
    model: String,
    key: String,
    prompt: String,
    consent: bool,
    message: String,
    job: Option<Job>,
    preview: Option<Preview>,
    texture: Option<egui::TextureHandle>,
}
impl Default for Assistant {
    fn default() -> Self {
        Self {
            open: false,
            settings: None,
            provider: Provider::Ollama,
            endpoint: "http://127.0.0.1:11434/api/chat".into(),
            model: String::new(),
            key: String::new(),
            prompt: String::new(),
            consent: false,
            message: String::new(),
            job: None,
            preview: None,
            texture: None,
        }
    }
}

struct ConnectionSettings {
    provider: Provider,
    endpoint: String,
    model: String,
    key: String,
    error: String,
}

impl Assistant {
    fn edit_settings(&mut self) {
        self.settings = Some(ConnectionSettings {
            provider: self.provider,
            endpoint: self.endpoint.clone(),
            model: self.model.clone(),
            key: self.key.clone(),
            error: String::new(),
        });
    }

    fn save_settings(&mut self) -> Result<(), String> {
        let draft = self.settings.as_ref().ok_or("No settings to save.")?;
        if draft.provider != Provider::Agent {
            endpoint(draft.endpoint.trim())?;
            if draft.model.trim().is_empty() {
                return Err("Enter a model ID for this connection.".into());
            }
        }
        if draft.model.len() > 200 || draft.key.len() > 4096 {
            return Err("Model or API key exceeds its size limit.".into());
        }
        let draft = self.settings.take().unwrap();
        if self.provider != draft.provider
            || self.endpoint != draft.endpoint
            || self.model != draft.model
        {
            self.consent = false;
        }
        self.provider = draft.provider;
        self.endpoint = draft.endpoint.trim().into();
        self.model = draft.model.trim().into();
        self.key = draft.key;
        self.message.clear();
        Ok(())
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.settings else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut cancel = false;
        egui::Window::new("Assistant settings")
            .id(egui::Id::new("assistant-settings"))
            .open(&mut open).default_width(390.0)
            .max_width((ctx.content_rect().width() - 32.0).max(180.0))
            .vscroll(true)
            .show(ctx, |ui| {
                ui.label("Connection");
                let old = draft.provider;
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut draft.provider, Provider::Agent, "Omarchy agent");
                    ui.selectable_value(&mut draft.provider, Provider::Ollama, "Ollama");
                    ui.selectable_value(&mut draft.provider, Provider::Compatible, "API");
                });
                if old != draft.provider {
                    draft.endpoint = match draft.provider {
                        Provider::Agent => "",
                        Provider::Ollama => "http://127.0.0.1:11434/api/chat",
                        Provider::Compatible => "https://api.openai.com/v1/chat/completions",
                    }.into();
                    draft.model.clear(); draft.key.clear(); draft.error.clear();
                }
                ui.separator();
                if draft.provider == Provider::Agent {
                    ui.label("Uses Omarchy's default agent (Codex supported). Sign in to Codex first; no API key needed here.");
                    ui.small("Reuses login, not custom configuration. Read-only mode with shell tools disabled.");
                } else {
                    ui.label("Full endpoint URL");
                    ui.text_edit_singleline(&mut draft.endpoint);
                }
                ui.label(if draft.provider == Provider::Agent { "Model override (optional)" } else { "Model ID (required)" });
                ui.text_edit_singleline(&mut draft.model);
                if draft.provider == Provider::Agent {
                    ui.small("Blank uses gpt-5.6-sol. Agent generation uses low reasoning for shorter waits.");
                }
                if draft.provider != Provider::Agent {
                    ui.label("API key (optional for local services)");
                    ui.add(egui::TextEdit::singleline(&mut draft.key).password(true));
                    if ui.button("Clear key").clicked() { draft.key.clear(); }
                }
                ui.small("Settings are saved for this Mote session only. API keys are never written to disk.");
                if !draft.error.is_empty() { ui.colored_label(ui.visuals().error_fg_color, &draft.error); }
                ui.separator();
                ui.horizontal(|ui| {
                    save = ui.button("Save and close").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if !open || cancel {
            self.settings = None;
        } else if save && let Err(error) = self.save_settings() {
            self.settings.as_mut().unwrap().error = error;
        }
    }

    fn validate_fields(&self) -> Result<(), String> {
        if self.provider != Provider::Agent && self.model.trim().is_empty() {
            return Err("Model is missing. Enter the exact model ID from your provider.".into());
        }
        if self.model.len() > 200 {
            return Err("Model ID is too long (maximum 200 bytes).".into());
        }
        if self.prompt.trim().is_empty() {
            return Err(
                "Prompt is empty. Type what you want to draw, or click Use example.".into(),
            );
        }
        if self.prompt.len() > 4000 {
            return Err(
                "Prompt is too long (maximum 4000 bytes). Shorten your description.".into(),
            );
        }
        if self.key.len() > 4096 {
            return Err("API key is too long (maximum 4096 bytes). Check what you pasted.".into());
        }
        if !self.consent {
            return Err("Permission is required. Check the box to send your prompt, dimensions and palette.".into());
        }
        if self.provider != Provider::Agent {
            endpoint(self.endpoint.trim())?;
        }
        Ok(())
    }
}
struct Target {
    id: u64,
    frame: usize,
    original: Document,
}
struct Job {
    cancel: Arc<AtomicBool>,
    target: Target,
    receiver: mpsc::Receiver<Result<Document, String>>,
    canceled: bool,
}
struct Preview {
    target: Target,
    result: Document,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artwork {
    layers: Vec<ArtLayer>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtLayer {
    name: String,
    rects: Vec<ArtRect>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtRect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: usize,
}

fn validate_canvas(doc: &Document, frame: usize) -> Result<(), String> {
    doc.validate()?;
    if doc.width > 128 || doc.height > 128 || frame >= doc.frames.len() || doc.palette.is_empty() {
        return Err(
            "AI generation currently supports canvases up to 128×128 with a nonempty palette."
                .into(),
        );
    }
    Ok(())
}

fn render_art(doc: &Document, frame: usize, text: &str) -> Result<Document, String> {
    validate_canvas(doc, frame)?;
    if text.len() > MAX_RESPONSE as usize {
        return Err("Generated artwork is too large.".into());
    }
    let art: Artwork = serde_json::from_str(text).map_err(
        |_| "Model returned invalid artwork JSON. Try a simpler prompt or another model.",
    )?;
    if art.layers.is_empty()
        || art.layers.len() > 8
        || !doc.can_grow(doc.layers.len() + art.layers.len(), doc.frames.len())
    {
        return Err("Generated layers exceed the project limits (maximum 8 new layers).".into());
    }
    let mut count = 0usize;
    let mut pixels = 0usize;
    for layer in &art.layers {
        if layer.name.trim().is_empty()
            || layer.name.len() > 80
            || layer.name.chars().any(char::is_control)
        {
            return Err("Generated layer name is invalid.".into());
        }
        for r in &layer.rects {
            count += 1;
            if r.w == 0
                || r.h == 0
                || r.x.checked_add(r.w).is_none_or(|x| x > doc.width)
                || r.y.checked_add(r.h).is_none_or(|y| y > doc.height)
                || r.color >= doc.palette.len()
            {
                return Err("Generated rectangle is outside the canvas or palette.".into());
            }
            pixels += r.w * r.h;
            if count > 4096 || pixels > 1_048_576 {
                return Err("Generated drawing exceeds the operation limit.".into());
            }
        }
    }
    if count == 0 {
        return Err("The model returned no drawing operations.".into());
    }
    let mut result = doc.clone();
    for layer in art.layers {
        let index = result.layers.len();
        result.layers.push(Layer {
            name: format!("AI · {}", layer.name),
            visible: true,
            locked: false,
            opacity: 255,
        });
        for f in &mut result.frames {
            f.cels.push(vec![CLEAR; doc.width * doc.height]);
        }
        for r in layer.rects {
            for y in r.y..r.y + r.h {
                for x in r.x..r.x + r.w {
                    result.frames[frame].cels[index][y * doc.width + x] = doc.palette[r.color];
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}

fn endpoint(text: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(text).map_err(|_| "Enter a valid API endpoint URL.")?;
    let local = matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "localhost"));
    if text.len() > 2048
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (url.scheme() == "http" && local))
    {
        return Err("Use HTTPS, or HTTP on localhost only. Put credentials in the API key field, not the URL.".into());
    }
    Ok(url)
}

fn request_body(provider: Provider, model: &str, prompt: &str, doc: &Document) -> Value {
    let system = "You are a pixel artist. Return ONLY a JSON object: {\"layers\":[{\"name\":\"Subject\",\"rects\":[{\"x\":0,\"y\":0,\"w\":1,\"h\":1,\"color\":0}]}]}. Each rect paints a filled axis-aligned rectangle using a zero-based palette index. Coordinates start at top-left. Use 1x1 rectangles for individual pixels, wider rectangles for runs. Unpainted pixels are transparent. Rectangles must fit the canvas. Layers are ordered bottom to top. Use 1–8 meaningful layers, at most 4096 rectangles total. No code, explanations, extra fields, animations or new colors. Make an intentional, readable pixel-art composition with silhouette, shading and highlights, not a rough diagram.";
    let messages = json!([
        {"role":"system", "content":system},
        {"role":"user", "content":json!({"prompt":prompt,"width":doc.width,"height":doc.height,"palette_rgba":doc.palette}).to_string()}
    ]);
    match provider {
        Provider::Agent => json!({"messages":messages}),
        Provider::Ollama => {
            json!({"model":model,"messages":messages,"stream":false,"format":"json","options":{"num_predict":8192}})
        }
        Provider::Compatible => {
            json!({"model":model,"messages":messages,"stream":false,"response_format":{"type":"json_object"},"max_completion_tokens":8192})
        }
    }
}

fn generate(
    provider: Provider,
    url: reqwest::Url,
    model: String,
    key: String,
    prompt: String,
    doc: &Document,
    frame: usize,
) -> Result<Document, String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| "Could not initialize the HTTP client.")?;
    let mut request = client
        .post(url)
        .json(&request_body(provider, &model, &prompt, doc));
    if !key.is_empty() {
        request = request.bearer_auth(key);
    }
    let response = request.send().map_err(
        |_| "Connection failed or timed out. Check the endpoint and running model service.",
    )?;
    if !response.status().is_success() {
        return Err(format!(
            "Provider returned HTTP {}. Check model, credentials and JSON-mode support.",
            response.status().as_u16()
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read provider response.")?;
    if bytes.len() > MAX_RESPONSE as usize {
        return Err("Provider response exceeds 2 MB.".into());
    }
    let response: Value =
        serde_json::from_slice(&bytes).map_err(|_| "Provider returned invalid JSON.")?;
    let content = match provider {
        Provider::Agent => return Err("Agent requests must use the process adapter.".into()),
        Provider::Ollama => {
            if response["done"] != true || response["done_reason"] == "length" {
                return Err("Generation did not finish. Try a simpler prompt.".into());
            }
            response["message"]["content"].as_str()
        }
        Provider::Compatible => {
            if response["choices"][0]["finish_reason"] != "stop" {
                return Err("Generation was truncated or refused. Try a simpler prompt.".into());
            }
            response["choices"][0]["message"]["content"].as_str()
        }
    }
    .ok_or("Provider response did not contain artwork.")?;
    render_art(doc, frame, content)
}

impl Studio {
    pub fn ai_busy(&self) -> bool {
        self.ai.job.is_some()
    }

    pub fn poll_ai(&mut self) {
        let Some(job) = &self.ai.job else {
            return;
        };
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("Generation worker stopped unexpectedly.".into())
            }
        };
        let job = self.ai.job.take().unwrap();
        if job.canceled {
            self.ai.message = "Canceled; result discarded.".into();
            return;
        }
        match result {
            Ok(result) => {
                self.ai.preview = Some(Preview {
                    target: job.target,
                    result,
                });
                self.ai.texture = None;
                self.ai.message = "Preview ready. Review before applying.".into();
            }
            Err(e) => self.ai.message = e,
        }
    }

    fn start_ai(&mut self) -> Result<(), String> {
        if self.ai.job.is_some() || self.io_job.is_some() {
            return Err("Another operation is still running.".into());
        }
        self.finish_gesture();
        validate_canvas(&self.doc, self.frame)?;
        self.ai.validate_fields()?;
        let url = if self.ai.provider == Provider::Agent {
            None
        } else {
            Some(endpoint(self.ai.endpoint.trim())?)
        };
        let target = Target {
            id: self.document_id,
            frame: self.frame,
            original: self.doc.clone(),
        };
        let doc = target.original.clone();
        let frame = target.frame;
        let provider = self.ai.provider;
        let model = self.ai.model.trim().to_owned();
        let key = self.ai.key.clone();
        let prompt = self.ai.prompt.clone();
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        std::thread::Builder::new()
            .name("mote-ai".into())
            .spawn(move || {
                let result = if provider == Provider::Agent {
                    let body = request_body(provider, &model, &prompt, &doc);
                    let input = format!(
                        "{}\n{}",
                        body["messages"][0]["content"].as_str().unwrap(),
                        body["messages"][1]["content"].as_str().unwrap()
                    );
                    crate::agent::generate(input, model, worker_cancel)
                        .and_then(|text| render_art(&doc, frame, &text))
                } else {
                    generate(provider, url.unwrap(), model, key, prompt, &doc, frame)
                };
                let _ = sender.send(result);
            })
            .map_err(|_| "Could not start generation worker.")?;
        self.ai.job = Some(Job {
            cancel,
            target,
            receiver,
            canceled: false,
        });
        self.ai.preview = None;
        self.ai.texture = None;
        self.ai.message =
            "Generating… You can continue editing; changed documents cannot accept stale previews."
                .into();
        Ok(())
    }

    fn apply_ai(&mut self) -> Result<(), String> {
        self.finish_gesture();
        let preview = self.ai.preview.as_ref().ok_or("No preview to apply.")?;
        if self.document_id != preview.target.id || self.doc != preview.target.original {
            return Err(
                "Return to the unchanged source tab, or regenerate for the current artwork.".into(),
            );
        }
        let preview = self.ai.preview.take().unwrap();
        self.checkpoint();
        self.doc = preview.result;
        self.frame = preview.target.frame;
        self.layer = self.doc.layers.len() - 1;
        self.playing = false;
        self.ai.texture = None;
        self.status = "AI artwork applied to new layers · Undo to revert".into();
        Ok(())
    }

    fn preview_source_index(&self) -> Option<usize> {
        let id = self.ai.preview.as_ref()?.target.id;
        if self.document_id == id {
            return Some(self.active_document);
        }
        self.documents
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| s.document_id == id))
    }

    fn open_ai_preview(&mut self) {
        let Some(preview) = self.ai.preview.take() else {
            return;
        };
        self.add_document(preview.result, None, "Generated artwork".into());
        // A generated tab must be considered unsaved even before the first edit.
        self.saved = Document::new(1, 1);
        self.frame = preview.target.frame;
        self.layer = self.doc.layers.len() - 1;
        self.ai.texture = None;
        self.ai.message = "Preview opened in a new tab. Your other tabs are unchanged.".into();
    }

    pub fn ai_window(&mut self, ctx: &egui::Context) {
        if !self.ai.open {
            return;
        }
        let mut open = true;
        let screen = ctx.content_rect();
        egui::Window::new("Assistant · Generate pixel art")
            .id(egui::Id::new("assistant-v2"))
            .open(&mut open)
            .default_pos(screen.min + egui::vec2(24.0, 80.0))
            .default_width(430.0)
            .max_width((screen.width() - 32.0).max(180.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let name = match self.ai.provider { Provider::Agent => "Omarchy agent", Provider::Ollama => "Ollama", Provider::Compatible => "API" };
                    ui.label(name);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(self.ai.job.is_none(), egui::Button::new("⚙ Settings")).clicked() { self.ai.edit_settings(); }
                    });
                });
                if !self.ai.message.is_empty() { ui.label(egui::RichText::new(&self.ai.message).strong()); }
                if let Some(job) = &mut self.ai.job {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        if ui.add_enabled(!job.canceled, egui::Button::new("Cancel / discard result")).clicked() {
                            job.canceled = true;
                            job.cancel.store(true, Ordering::Relaxed);
                            self.ai.message = "Cancel requested — waiting for the request to end; result will be discarded.".into();
                        }
                    });
                }
                if self.ai.preview.is_some() {
                    let preview = self.ai.preview.as_ref().unwrap();
                    let can_apply = self.document_id == preview.target.id && self.doc == preview.target.original;
                    ui.small(format!("Preview canvas: {} × {} · frame {}", preview.result.width, preview.result.height, preview.target.frame + 1));
                    if !can_apply {
                        ui.label("This preview belongs to a different or changed canvas. Return to its source, or open the preview separately.");
                        if let Some(index) = self.preview_source_index()
                            && index != self.active_document
                            && ui.button(format!("Go to source: {}", self.document_title(index))).clicked() {
                                self.switch_document(index);
                        }
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(can_apply && self.io_job.is_none() && self.pending.is_none(), egui::Button::new("Apply as new layers")).clicked() {
                            self.ai.message = match self.apply_ai() { Ok(()) => "Applied. One undo step.".into(), Err(e) => e };
                        }
                        if ui.add_enabled(self.io_job.is_none() && self.pending.is_none(), egui::Button::new("Open preview in new tab")).on_hover_text("Opens the complete preview, including the original artwork snapshot, without changing other tabs.").clicked() {
                            self.open_ai_preview();
                        }
                        if ui.button("Discard preview").clicked() {
                            self.ai.preview = None;
                            self.ai.texture = None;
                            self.ai.message = "Preview discarded. Your artwork is unchanged.".into();
                        }
                    });
                }
                ui.separator();
                let mut scroll = egui::ScrollArea::vertical().id_salt("assistant-content")
                    .max_height((screen.height() - 240.0).clamp(100.0, 580.0));
                if self.ai.preview.is_some() && self.ai.texture.is_none() {
                    scroll = scroll.vertical_scroll_offset(0.0);
                }
                scroll.show(ui, |ui| {
                if let Some(preview) = &self.ai.preview {
                    ui.separator();
                    ui.label(format!("Preview · {} new layers", preview.result.layers.len() - preview.target.original.layers.len()));
                    if self.ai.texture.is_none() {
                        let pixels = preview.result.composite(preview.target.frame);
                        let image = egui::ColorImage::from_rgba_unmultiplied([preview.result.width, preview.result.height], pixels.as_flattened());
                        self.ai.texture = Some(ctx.load_texture("ai-preview", image, egui::TextureOptions::NEAREST));
                    }
                    if let Some(tex) = &self.ai.texture {
                        let scale = (ui.available_width().min(320.0) / preview.result.width.max(preview.result.height) as f32).max(0.1);
                        ui.image((tex.id(), tex.size_vec2() * scale));
                    }
                    ui.small("Preview combines generated layers with the original artwork. Apply requires the source tab to be unchanged.");
                }

                ui.label(format!("Active canvas: {} × {} · {} colors · frame {}", self.doc.width, self.doc.height, self.doc.palette.len(), self.frame + 1));
                ui.small("New layers only. Existing artwork is not sent.");
                ui.add_enabled_ui(self.ai.job.is_none() && self.ai.settings.is_none(), |ui| {
                    if self.ai.provider != Provider::Agent && self.ai.model.is_empty() {
                        ui.small("Choose your connection and model in ⚙ Settings to get started.");
                    }
                    ui.label("What would you like to draw?");
                    ui.add(egui::TextEdit::multiline(&mut self.ai.prompt).desired_rows(6).desired_width(f32::INFINITY).hint_text("Describe your pixel art…"));
                    ui.horizontal_wrapped(|ui| {
                        ui.small(format!("{} / 4000 bytes", self.ai.prompt.len()));
                        if ui.add_enabled(self.ai.prompt.trim().is_empty(), egui::Button::new("Use example")).clicked() {
                            self.ai.prompt = "A tiny forest cabin, warm windows, mossy roof, transparent background".into();
                        }
                    });
                    ui.checkbox(&mut self.ai.consent, "Send my prompt, dimensions and palette to this provider/agent");
                    ui.small("Provider charges may apply. Local services may use cloud models; check your setup. No automatic retries.");
                    if ui.add_enabled(self.io_job.is_none() && self.pending.is_none(), egui::Button::new("Generate preview")).clicked()
                        && let Err(e) = self.start_ai() { self.ai.message = e; }
                });

                if self.ai.job.is_some() {
                    ui.small("Request timeout: 120 seconds. Cancel discards the result; the provider may continue processing and billing.");
                }
                    });
            });
        self.ai.open = open;
        if open {
            self.ai.settings_window(ctx);
        }
    }
}
