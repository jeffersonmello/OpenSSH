use super::*;
use egui::{
    epaint::Shape,
    text::{CCursor, CCursorRange},
    Event, FullOutput, Id, Key, Modifiers, RawInput, Rect,
};

const SAMPLE: &str = "ephemeral editor test text; no real key or credential";

fn render_secret(
    ctx: &egui::Context,
    text: &mut String,
    time: f64,
    events: Vec<Event>,
) -> (FullOutput, Id) {
    render_secret_mode(ctx, text, time, events, false)
}

fn render_secret_mode(
    ctx: &egui::Context,
    text: &mut String,
    time: f64,
    events: Vec<Event>,
    reveal: bool,
) -> (FullOutput, Id) {
    let mut id = None;
    let output = ctx.run(
        RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(600.0, 240.0),
            )),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = secret_input(ui, "reused-sensitive-editor", text, false, reveal, "");
                response.request_focus();
                id = Some(response.id);
            });
        },
    );
    (output, id.unwrap())
}

fn paint_text(shape: &Shape, rendered: &mut String) {
    match shape {
        Shape::Text(shape) => rendered.push_str(&shape.galley.job.text),
        Shape::Vec(shapes) => {
            for shape in shapes {
                paint_text(shape, rendered);
            }
        }
        _ => {}
    }
}

fn assert_no_clipboard_text(output: &FullOutput) {
    assert!(!output
        .platform_output
        .commands
        .iter()
        .any(|command| matches!(command, egui::OutputCommand::CopyText(_))));
}

#[test]
fn masked_secret_never_enters_paint_or_clipboard_output() {
    let ctx = egui::Context::default();
    let mut text = SAMPLE.to_owned();
    let (output, id) = render_secret(&ctx, &mut text, 0.0, vec![]);
    let mut painted = String::new();
    for shape in &output.shapes {
        paint_text(&shape.shape, &mut painted);
    }
    assert!(!painted.contains(SAMPLE));
    assert!(
        painted.contains('•'),
        "Expected masked glyphs in actual paint output"
    );

    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(text.chars().count()),
    )));
    state.store(&ctx, id);
    let (copied, _) = render_secret(&ctx, &mut text, 0.1, vec![Event::Copy]);
    assert_no_clipboard_text(&copied);
    assert_eq!(text, SAMPLE);

    let (cut, _) = render_secret(&ctx, &mut text, 0.2, vec![Event::Cut]);
    assert_no_clipboard_text(&cut);
    assert!(
        text.is_empty(),
        "Cut should remove the selected text without exporting it"
    );
}

#[test]
fn cleared_editor_id_cannot_restore_previous_secret_through_undo() {
    let ctx = egui::Context::default();
    let mut text = String::new();
    let (_, original_id) = render_secret(&ctx, &mut text, 0.0, vec![]);
    render_secret(&ctx, &mut text, 0.1, vec![Event::Paste(SAMPLE.into())]);
    assert_eq!(text, SAMPLE);
    // Wait past egui's undo stabilization interval without a wall-clock sleep.
    render_secret_mode(&ctx, &mut text, 2.0, vec![], true);
    let state = egui::TextEdit::load_state(&ctx, original_id).unwrap();
    let current = (
        CCursorRange::one(CCursor::new(text.chars().count())),
        text.clone(),
    );
    assert!(!state.undoer().has_undo(&current));

    text.zeroize();
    text.clear();
    let (_, reused_id) = render_secret(&ctx, &mut text, 2.1, vec![]);
    assert_eq!(
        original_id, reused_id,
        "Regression requires reuse of the same widget ID"
    );
    let command_z = Event::Key {
        key: Key::Z,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        },
    };
    render_secret(&ctx, &mut text, 2.2, vec![command_z]);
    assert!(
        text.is_empty(),
        "Undo must not recover a previous identity's secret"
    );
    let state = egui::TextEdit::load_state(&ctx, reused_id).unwrap();
    assert!(!state
        .undoer()
        .has_undo(&(CCursorRange::one(CCursor::new(0)), String::new())));
}

#[test]
fn existing_draft_preserves_identity_and_only_its_bound_hosts() {
    let workspace = Workspace::demo();
    let identity = &workspace.identities[0];
    let draft = KeyDraft::existing(identity, &workspace);
    assert_eq!(draft.id.as_ref(), Some(&identity.id));
    assert_eq!(draft.name, identity.name);
    assert_eq!(
        draft.linked_hosts,
        vec![workspace.hosts[0].id.clone(), workspace.hosts[1].id.clone()]
    );
    assert!(!draft.has_material());
    assert!(draft.private_key.is_empty());
    assert!(draft.passphrase.is_empty());
    assert!(!draft.reveal_key && !draft.reveal_passphrase && !draft.remember_passphrase);
}
