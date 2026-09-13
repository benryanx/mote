use super::*;
use std::io::Write;

const ART: &str = r#"{"layers":[{"name":"Flower","rects":[{"x":1,"y":1,"w":2,"h":2,"color":1}]}]}"#;

#[test]
fn validation_identifies_the_missing_or_oversized_field() {
    let mut ai = Assistant::default();
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("Model is missing")
    );
    ai.model = "test-model".into();
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("Prompt is empty")
    );
    ai.prompt = "A rose".into();
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("Permission is required")
    );
    ai.consent = true;
    assert!(ai.validate_fields().is_ok());
    ai.prompt = "é".repeat(2001);
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("Prompt is too long")
    );
    ai.prompt = "A rose".into();
    ai.key = "x".repeat(4097);
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("API key is too long")
    );
    ai.key.clear();
    ai.model = "x".repeat(201);
    assert!(
        ai.validate_fields()
            .unwrap_err()
            .starts_with("Model ID is too long")
    );
}

#[test]
fn invalid_form_never_starts_a_request_or_changes_artwork() {
    let mut app = Studio::default();
    app.ai.model = "test-model".into();
    app.ai.consent = true;
    assert!(app.start_ai().unwrap_err().starts_with("Prompt is empty"));
    assert!(!app.ai_busy());
    assert_eq!(app.doc, app.saved);
}

#[test]
fn new_layers_preserve_original_cels_and_other_frames() {
    let mut doc = Document::new(8, 8);
    doc.frames.push(doc.frames[0].clone());
    doc.layers[0].locked = true;
    let out = render_art(&doc, 1, ART).unwrap();
    assert_eq!(out.layers[0].locked, doc.layers[0].locked);
    assert_eq!(out.frames[0].cels[0], doc.frames[0].cels[0]);
    assert_eq!(out.frames[1].cels[0], doc.frames[1].cels[0]);
    assert!(out.frames[0].cels[1].iter().all(|p| *p == CLEAR));
    assert_eq!(out.frames[1].cels[1][9], doc.palette[1]);
    assert_eq!(out.frames.len(), 2);
}

#[test]
fn rejects_invalid_and_excessive_model_data() {
    let doc = Document::new(8, 8);
    for invalid in [
        ART.replace("\"x\":1", "\"x\":8"),
        ART.replace("\"w\":2", "\"w\":0"),
        ART.replace("\"color\":1", "\"color\":256"),
        ART.replace("\"name\":\"Flower\"", "\"name\":\"\""),
        ART.replace("\"x\":1", "\"x\":18446744073709551615"),
        "{\"layers\":[],\"code\":\"anything\"}".into(),
        "```json\n{}\n```".into(),
        json!({"layers":[{"name":"Too many", "rects":vec![json!({"x":0,"y":0,"w":1,"h":1,"color":0});4097]}]}).to_string(),
    ] {
        assert!(render_art(&doc, 0, &invalid).is_err());
    }
    assert!(render_art(&Document::new(129, 8), 0, ART).is_err());
    assert!(render_art(&doc, 20, ART).is_err());
}

#[test]
fn apply_is_atomic_undoable_and_rejects_wrong_or_changed_tab() {
    let mut app = Studio::default();
    let original = app.doc.clone();
    app.ai.preview = Some(Preview {
        result: render_art(&original, 0, ART).unwrap(),
        target: Target {
            id: app.document_id,
            frame: 0,
            original: original.clone(),
        },
    });
    app.add_document(Document::new(8, 8), None, "Other".into());
    assert!(app.apply_ai().is_err());
    app.switch_document(0);
    app.doc.put(0, 0, 0, 0, [255; 4]);
    assert!(app.apply_ai().is_err());
    app.doc = original.clone();
    app.apply_ai().unwrap();
    assert_eq!(app.undo.len(), 1);
    let generated = app.doc.clone();
    app.undo();
    assert_eq!(app.doc, original);
    app.redo();
    assert_eq!(app.doc, generated);
    assert!(app.ai.preview.is_none());
}

#[test]
fn preview_recovery_preserves_other_tabs() {
    let mut app = Studio::default();
    let original = app.doc.clone();
    let generated = render_art(&original, 0, ART).unwrap();
    app.ai.preview = Some(Preview {
        result: generated.clone(),
        target: Target {
            id: app.document_id,
            frame: 0,
            original: original.clone(),
        },
    });
    app.add_document(Document::new(64, 64), None, "Other".into());
    let other = app.doc.clone();
    assert_eq!(app.preview_source_index(), Some(0));
    app.open_ai_preview();
    assert_eq!(app.doc, generated);
    assert!(app.dirty());
    app.switch_document(1);
    assert_eq!(app.doc, other);
    app.switch_document(0);
    assert_eq!(app.doc, original);
}

#[test]
fn cancellation_ignores_completed_result_and_polls_without_blocking() {
    let mut app = Studio::default();
    let (sender, receiver) = mpsc::channel();
    app.ai.job = Some(Job {
        target: Target {
            id: app.document_id,
            frame: 0,
            original: app.doc.clone(),
        },
        receiver,
        canceled: true,
        cancel: Arc::new(AtomicBool::new(true)),
    });
    app.poll_ai();
    assert!(app.ai_busy());
    sender
        .send(Ok(render_art(&app.doc, 0, ART).unwrap()))
        .unwrap();
    app.poll_ai();
    assert!(!app.ai_busy());
    assert!(app.ai.preview.is_none());
    assert_eq!(app.doc, app.saved);
}

#[test]
fn agent_requires_prompt_and_consent_but_not_http_settings() {
    let mut assistant = Assistant {
        provider: Provider::Agent,
        endpoint: String::new(),
        prompt: "A rose".into(),
        ..Assistant::default()
    };
    assert!(assistant.validate_fields().is_err());
    assistant.consent = true;
    assert!(assistant.validate_fields().is_ok());
    assistant.prompt.clear();
    assert!(assistant.validate_fields().is_err());
}

#[test]
fn settings_are_staged_validated_and_reset_consent_on_connection_change() {
    let mut assistant = Assistant::default();
    assistant.edit_settings();
    assistant.settings.as_mut().unwrap().model = "draft-model".into();
    assert!(assistant.model.is_empty());
    assistant.settings = None;
    assert!(assistant.model.is_empty());
    assistant.edit_settings();
    assert!(assistant.save_settings().is_err());
    assert!(assistant.settings.is_some());
    assistant.consent = true;
    assistant.settings.as_mut().unwrap().provider = Provider::Agent;
    assistant.save_settings().unwrap();
    assert!(assistant.provider == Provider::Agent);
    assert!(!assistant.consent);
    assert!(assistant.settings.is_none());
}

#[test]
fn settings_window_renders_without_display() {
    let mut assistant = Assistant::default();
    assistant.edit_settings();
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        assistant.settings_window(ctx)
    });
    assert!(assistant.settings.is_some());
}

#[test]
fn endpoint_requires_tls_except_loopback_and_never_url_credentials() {
    for valid in [
        "http://127.0.0.1:11434/api/chat",
        "http://[::1]:11434/api/chat",
        "https://example.com/v1/chat/completions",
    ] {
        assert!(endpoint(valid).is_ok());
    }
    for invalid in [
        "http://example.com/api",
        "file:///etc/passwd",
        "https://user:secret@example.com/api",
        "https://example.com/api?key=secret",
        "https://example.com/api#secret",
    ] {
        assert!(endpoint(invalid).is_err());
    }
}

// Real HTTP exchange against a loopback fixture, never a paid provider.
fn mock_provider(status: &str, body: String) -> (reqwest::Url, std::thread::JoinHandle<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = endpoint(&format!("http://{}/chat", listener.local_addr().unwrap())).unwrap();
    let status = status.to_owned();
    let handle = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut block = [0; 4096];
            let n = socket.read(&mut block).unwrap();
            if n == 0 {
                break;
            }
            request.extend_from_slice(&block[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&request[..end]).to_lowercase();
                let len: usize = header
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + len {
                    break;
                }
            }
        }
        write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        String::from_utf8(request).unwrap()
    });
    (url, handle)
}

#[test]
fn both_provider_adapters_generate_using_minimal_context() {
    for provider in [Provider::Ollama, Provider::Compatible] {
        let body = match provider {
            Provider::Agent => unreachable!(),
            Provider::Ollama => json!({"done":true,"message":{"content":ART}}),
            Provider::Compatible => {
                json!({"choices":[{"finish_reason":"stop","message":{"content":ART}}]})
            }
        };
        let (url, worker) = mock_provider("200 OK", body.to_string());
        let mut doc = Document::new(8, 8);
        doc.layers[0].name = "private layer title".into();
        let out = generate(
            provider,
            url,
            "test-model".into(),
            "test-key".into(),
            "Draw a flower".into(),
            &doc,
            0,
        )
        .unwrap();
        assert_eq!(out.layers.len(), 2);
        let request = worker.join().unwrap();
        assert!(request.contains("Draw a flower"));
        assert!(!request.contains("private layer title"));
        assert!(request.contains("Bearer test-key"));
    }
}

#[test]
fn http_errors_do_not_echo_secrets_and_truncation_is_rejected() {
    for (status, body) in [
        ("401 Unauthorized", "secret-api-key".into()),
        (
            "200 OK",
            json!({"choices":[{"finish_reason":"length","message":{"content":ART}}]}).to_string(),
        ),
    ] {
        let (url, worker) = mock_provider(status, body);
        let error = generate(
            Provider::Compatible,
            url,
            "test".into(),
            String::new(),
            "flower".into(),
            &Document::new(8, 8),
            0,
        )
        .unwrap_err();
        assert!(!error.contains("secret-api-key"));
        worker.join().unwrap();
    }
}

#[test]
fn assistant_window_renders_with_preview() {
    let mut app = Studio::default();
    app.ai.open = true;
    app.ai.preview = Some(Preview {
        target: Target {
            id: app.document_id,
            frame: 0,
            original: app.doc.clone(),
        },
        result: render_art(&app.doc, 0, ART).unwrap(),
    });
    let ctx = egui::Context::default();
    let out = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(640.0, 480.0),
            )),
            ..Default::default()
        },
        |ctx| app.ai_window(ctx),
    );
    assert!(!out.shapes.is_empty());
    assert_eq!(app.doc, app.saved);
}

#[test]
fn oversized_responses_and_redirect_status_are_rejected() {
    for (status, body) in [
        ("200 OK", " ".repeat(MAX_RESPONSE as usize + 1)),
        ("302 Found", "redirect response".into()),
    ] {
        let (url, worker) = mock_provider(status, body);
        assert!(
            generate(
                Provider::Ollama,
                url,
                "test".into(),
                String::new(),
                "flower".into(),
                &Document::new(8, 8),
                0
            )
            .is_err()
        );
        worker.join().unwrap();
    }
}
