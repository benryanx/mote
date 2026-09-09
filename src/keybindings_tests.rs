use super::*;
#[test]
fn settings_and_export_dialogs_render_at_small_window_size() {
    for keyboard in [true, false] {
        let ctx = egui::Context::default();
        let mut app = Studio::default();
        let original = app.doc.clone();
        if keyboard {
            app.key_editor.open = true;
        } else {
            app.open_export();
        }
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 600.0),
                )),
                ..Default::default()
            },
            |ctx| app.dialogs(ctx),
        );
        assert!(!output.shapes.is_empty());
        assert_eq!(app.doc, original);
    }
}
#[test]
fn defaults_roundtrip_and_reject_conflicts() {
    let mut b = defaults();
    assert!(valid(&b));
    let json = serde_json::to_string(&b).unwrap();
    assert!(valid(&serde_json::from_str::<Vec<Binding>>(&json).unwrap()));
    b[1].keys = b[0].keys.clone();
    assert!(!valid(&b));
}
#[test]
fn remapping_removes_old_key_and_keeps_exact_modifiers() {
    let mut app = Studio::default();
    let binding = app
        .bindings
        .iter_mut()
        .find(|b| b.command == Command::NewLayer)
        .unwrap();
    binding.keys = vec![Chord::parse("Ctrl+J")];
    let ctx = egui::Context::default();
    for (chord, count) in [
        (Chord::parse("Shift+N"), 1),
        (Chord::parse("Ctrl+Alt+J"), 1),
        (Chord::parse("Ctrl+J"), 2),
    ] {
        let modifiers = egui::Modifiers {
            ctrl: chord.ctrl,
            command: chord.ctrl,
            alt: chord.alt,
            shift: chord.shift,
            ..Default::default()
        };
        let _ = ctx.run(
            egui::RawInput {
                modifiers,
                events: vec![egui::Event::Key {
                    key: chord.key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                ..Default::default()
            },
            |ctx| app.shortcuts(ctx),
        );
        assert_eq!(app.doc.layers.len(), count);
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::Key {
                    key: chord.key,
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
    app.undo();
    assert_eq!(app.doc.layers.len(), 1);
}
#[test]
fn layer_duplication_preserves_every_frame_and_undo() {
    let mut app = Studio::default();
    app.doc.put(0, 0, 1, 1, [9; 4]);
    app.doc.frames.push(app.doc.frames[0].clone());
    let before = app.doc.clone();
    app.add_layer(true);
    assert_eq!(app.doc.layers.len(), 2);
    for f in &app.doc.frames {
        assert_eq!(f.cels[0], f.cels[1]);
    }
    app.undo();
    assert_eq!(app.doc, before);
}
