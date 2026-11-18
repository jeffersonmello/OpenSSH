//! A PTY-backed, bounded VT100 terminal. Terminal contents never enter logs.

use crate::{
    connection,
    models::{Host, Identity, Tunnel},
};
use anyhow::{anyhow, ensure, Context, Result};
use eframe::egui::{
    self, Align2, Color32, Event, FontId, Key, Modifiers, Rect, Response, Sense, Stroke, Vec2,
};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::{
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const INITIAL_ROWS: u16 = 28;
const INITIAL_COLS: u16 = 100;
const SCROLLBACK_LINES: usize = 2_000;
const MAX_INPUT_BYTES: usize = 65_536;

pub struct TerminalSession {
    pub title: String,
    master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    input: Option<SyncSender<Vec<u8>>>,
    parser: Arc<Mutex<vt100::Parser>>,
    repaint: Arc<Mutex<Option<egui::Context>>>,
    reader_finished: Arc<AtomicBool>,
    rows: u16,
    cols: u16,
    finished: bool,
    focus_next_frame: bool,
    last_error: Option<String>,
}

impl TerminalSession {
    pub fn ssh(host: &Host, identity: Option<&Identity>) -> Result<Self> {
        let args = connection::ssh_arguments(host, identity)?;
        let mut command = CommandBuilder::new(connection::ssh_executable());
        command.args(args);
        Self::spawn(host.name.clone(), command)
    }

    pub fn tunnel(host: &Host, identity: Option<&Identity>, tunnel: &Tunnel) -> Result<Self> {
        let args = connection::tunnel_arguments(host, identity, tunnel)?;
        let mut command = CommandBuilder::new(connection::ssh_executable());
        command.args(args);
        Self::spawn(
            format!("{} · {}", tunnel.kind.label(), tunnel.name),
            command,
        )
    }

    pub fn local_shell() -> Result<Self> {
        let shell = if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/sh"
        };
        let mut command = CommandBuilder::new(shell);
        command.arg("-l");
        Self::spawn("Terminal local".into(), command)
    }

    fn spawn(title: String, mut command: CommandBuilder) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: INITIAL_ROWS,
                cols: INITIAL_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("Não foi possível criar o terminal PTY")?;
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;
        let child = pair
            .slave
            .spawn_command(command)
            .context("Não foi possível iniciar a sessão")?;
        drop(pair.slave);
        let parser = Arc::new(Mutex::new(vt100::Parser::new(
            INITIAL_ROWS,
            INITIAL_COLS,
            SCROLLBACK_LINES,
        )));
        let reader_parser = Arc::clone(&parser);
        let repaint = Arc::new(Mutex::new(None::<egui::Context>));
        let reader_repaint = Arc::clone(&repaint);
        let reader_finished = Arc::new(AtomicBool::new(false));
        let reader_done = Arc::clone(&reader_finished);
        thread::spawn(move || {
            let mut buffer = [0u8; 8_192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let Ok(mut parser) = reader_parser.lock() else {
                            break;
                        };
                        parser.process(&buffer[..count]);
                        drop(parser);
                        if let Ok(context) = reader_repaint.lock() {
                            if let Some(context) = context.as_ref() {
                                context.request_repaint();
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
            reader_done.store(true, Ordering::Release);
            if let Ok(context) = reader_repaint.lock() {
                if let Some(context) = context.as_ref() {
                    context.request_repaint();
                }
            }
        });
        // Slow or blocked remote input cannot block the UI thread. The bounded
        // queue and per-message cap also bound queued paste memory.
        let (input, receiver) = mpsc::sync_channel::<Vec<u8>>(32);
        thread::spawn(move || {
            while let Ok(bytes) = receiver.recv() {
                if writer
                    .write_all(&bytes)
                    .and_then(|_| writer.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        Ok(Self {
            title,
            master: Some(pair.master),
            child: Some(child),
            input: Some(input),
            parser,
            repaint,
            reader_finished,
            rows: INITIAL_ROWS,
            cols: INITIAL_COLS,
            finished: false,
            focus_next_frame: true,
            last_error: None,
        })
    }

    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        ensure!(
            bytes.len() <= MAX_INPUT_BYTES,
            "A entrada excede o limite de 64 KiB por envio."
        );
        let input = self.input.as_ref().context("A sessão foi encerrada")?;
        input.try_send(bytes.to_vec()).map_err(|error| match error {
            TrySendError::Full(_) => {
                anyhow!("O terminal está ocupado. Aguarde antes de enviar mais dados.")
            }
            TrySendError::Disconnected(_) => anyhow!("A sessão não aceita mais entrada."),
        })
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        let rows = rows.clamp(2, 200);
        let cols = cols.clamp(10, 400);
        if (rows, cols) == (self.rows, self.cols) {
            return Ok(());
        }
        if let Some(master) = self.master.as_ref() {
            master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        self.parser
            .lock()
            .map_err(|_| anyhow!("Estado do terminal indisponível"))?
            .set_size(rows, cols);
        self.rows = rows;
        self.cols = cols;
        Ok(())
    }

    pub fn screen_text(&self) -> String {
        self.parser
            .lock()
            .map(|parser| parser.screen().contents())
            .unwrap_or_default()
    }

    pub fn is_finished(&mut self) -> bool {
        if !self.finished {
            if let Some(child) = self.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(_)) => self.finished = true,
                    Ok(None) => {}
                    Err(error) => {
                        self.finished = true;
                        self.last_error = Some(format!("Falha ao consultar a sessão: {error}"));
                    }
                }
            }
        }
        self.finished
    }

    pub fn close(&mut self) {
        self.input.take();
        self.master.take();
        if let Some(child) = self.child.take() {
            // Reap on a worker so a process slow to stop never freezes a window.
            thread::spawn(move || reap_child(child));
        }
        self.finished = true;
    }

    pub fn render(&mut self, ui: &mut egui::Ui) -> Response {
        if let Ok(mut context) = self.repaint.lock() {
            *context = Some(ui.ctx().clone());
        }
        let font = FontId::monospace(13.0);
        let fg = Color32::from_rgb(211, 222, 235);
        let bg = Color32::from_rgb(12, 17, 26);
        let glyph = ui.painter().layout_no_wrap("M".into(), font.clone(), fg);
        let cell_size = Vec2::new(glyph.size().x.max(7.0), glyph.size().y.max(18.0));
        let available = ui.available_size().max(Vec2::new(120.0, 80.0));
        let (rect, response) = ui.allocate_exact_size(available, Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::Text);
        if response.clicked() || self.focus_next_frame {
            response.request_focus();
            self.focus_next_frame = false;
        }
        if response.has_focus() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                )
            });
        }
        ui.painter().rect_filled(rect, 8.0, bg);
        let inner = rect.shrink(12.0);
        let rows = (inner.height() / cell_size.y).floor().max(2.0) as u16;
        let cols = (inner.width() / cell_size.x).floor().max(10.0) as u16;
        if let Err(error) = self.resize(rows, cols) {
            self.last_error = Some(error.to_string());
        }
        self.is_finished();

        let screen = self
            .parser
            .lock()
            .ok()
            .map(|parser| parser.screen().clone());
        if let Some(screen) = screen {
            let focused = response.has_focus();
            if focused {
                let events = ui.input(|input| input.events.clone());
                let modifiers = ui.input(|input| input.modifiers);
                for event in events {
                    let bytes = match event {
                        Event::Text(text) if !modifiers.ctrl && !modifiers.mac_cmd => {
                            Some(text.into_bytes())
                        }
                        Event::Paste(text) => Some(paste_bytes(&text, screen.bracketed_paste())),
                        Event::Copy => {
                            ui.ctx().copy_text(screen.contents());
                            None
                        }
                        Event::Key {
                            key,
                            pressed: true,
                            modifiers,
                            ..
                        } => key_bytes(key, modifiers, screen.application_cursor()),
                        _ => None,
                    };
                    if let Some(bytes) = bytes {
                        if let Ok(mut parser) = self.parser.lock() {
                            parser.set_scrollback(0);
                        }
                        ui.ctx().request_repaint();
                        if let Err(error) = self.write(&bytes) {
                            self.last_error = Some(error.to_string());
                        }
                    }
                }
            }
            if response.hovered() {
                let delta = ui.input(|input| input.raw_scroll_delta.y);
                if delta != 0.0 {
                    if let Ok(mut parser) = self.parser.lock() {
                        let current = parser.screen().scrollback();
                        let offset = ((current as f32) + delta / 8.0).max(0.0) as usize;
                        parser.set_scrollback(offset);
                        ui.ctx().request_repaint();
                    }
                }
            }
            let painter = ui.painter().with_clip_rect(inner);
            for row in 0..self.rows {
                for col in 0..self.cols {
                    let Some(cell) = screen.cell(row, col) else {
                        continue;
                    };
                    if cell.is_wide_continuation() {
                        continue;
                    }
                    let position = inner.min
                        + Vec2::new(f32::from(col) * cell_size.x, f32::from(row) * cell_size.y);
                    let mut cell_fg = terminal_color(cell.fgcolor(), fg, cell.bold());
                    let mut cell_bg = terminal_color(cell.bgcolor(), bg, false);
                    if cell.inverse() {
                        std::mem::swap(&mut cell_fg, &mut cell_bg);
                    }
                    let width = if cell.is_wide() { 2.0 } else { 1.0 };
                    if cell_bg != bg {
                        painter.rect_filled(
                            Rect::from_min_size(
                                position,
                                Vec2::new(cell_size.x * width, cell_size.y),
                            ),
                            0.0,
                            cell_bg,
                        );
                    }
                    if cell.has_contents() {
                        painter.text(
                            position,
                            Align2::LEFT_TOP,
                            cell.contents(),
                            font.clone(),
                            cell_fg,
                        );
                    }
                    if cell.underline() {
                        painter.line_segment(
                            [
                                position + Vec2::new(0.0, cell_size.y - 2.0),
                                position + Vec2::new(cell_size.x * width, cell_size.y - 2.0),
                            ],
                            Stroke::new(1.0_f32, cell_fg),
                        );
                    }
                }
            }
            if focused && !self.finished && !screen.hide_cursor() && screen.scrollback() == 0 {
                let (row, col) = screen.cursor_position();
                let origin = inner.min
                    + Vec2::new(f32::from(col) * cell_size.x, f32::from(row) * cell_size.y);
                painter.rect_filled(
                    Rect::from_min_size(origin, cell_size),
                    0.0,
                    Color32::from_rgba_unmultiplied(116, 228, 196, 70),
                );
            }
        }
        if let Some(error) = self.last_error.as_ref() {
            ui.painter().text(
                rect.left_bottom() + Vec2::new(12.0, -4.0),
                Align2::LEFT_BOTTOM,
                error,
                FontId::proportional(12.0),
                Color32::from_rgb(248, 168, 116),
            );
        }
        // Output wakes egui from the reader thread. Only poll briefly for child
        // termination after EOF, so idle sessions do not redraw continuously.
        if !self.finished && self.reader_finished.load(Ordering::Acquire) {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }
        response
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        self.close();
    }
}

fn reap_child(mut child: Box<dyn Child + Send + Sync>) {
    // portable-pty's Unix kill sends a signal to the numeric PID before looking
    // at cached exit status. A reaped child's PID may already have been reused.
    // Only signal a child still owned by this handle and known to be running.
    if matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    // Do not permit clipboard escape sequences to terminate a bracketed paste.
    let sanitized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace(['\x1b', '\0'], "");
    if bracketed {
        format!("\x1b[200~{sanitized}\x1b[201~").into_bytes()
    } else {
        sanitized.into_bytes()
    }
}

fn key_bytes(key: Key, modifiers: Modifiers, application_cursor: bool) -> Option<Vec<u8>> {
    if modifiers.mac_cmd || (modifiers.command && !modifiers.ctrl) {
        return None;
    }
    if modifiers.ctrl {
        let control = match key {
            Key::A => 1,
            Key::B => 2,
            Key::C => 3,
            Key::D => 4,
            Key::E => 5,
            Key::F => 6,
            Key::G => 7,
            Key::H => 8,
            Key::I => 9,
            Key::J => 10,
            Key::K => 11,
            Key::L => 12,
            Key::M => 13,
            Key::N => 14,
            Key::O => 15,
            Key::P => 16,
            Key::Q => 17,
            Key::R => 18,
            Key::S => 19,
            Key::T => 20,
            Key::U => 21,
            Key::V => 22,
            Key::W => 23,
            Key::X => 24,
            Key::Y => 25,
            Key::Z => 26,
            Key::Space => 0,
            _ => return None,
        };
        return Some(vec![control]);
    }
    let arrow_prefix = if application_cursor { "\x1bO" } else { "\x1b[" };
    let bytes = match key {
        Key::Enter => "\r".into(),
        Key::Backspace => "\x7f".into(),
        Key::Escape => "\x1b".into(),
        Key::Tab => {
            if modifiers.shift {
                "\x1b[Z".into()
            } else {
                "\t".into()
            }
        }
        Key::ArrowUp => format!("{arrow_prefix}A"),
        Key::ArrowDown => format!("{arrow_prefix}B"),
        Key::ArrowRight => format!("{arrow_prefix}C"),
        Key::ArrowLeft => format!("{arrow_prefix}D"),
        Key::Home => format!("{arrow_prefix}H"),
        Key::End => format!("{arrow_prefix}F"),
        Key::Insert => "\x1b[2~".into(),
        Key::Delete => "\x1b[3~".into(),
        Key::PageUp => "\x1b[5~".into(),
        Key::PageDown => "\x1b[6~".into(),
        Key::F1 => "\x1bOP".into(),
        Key::F2 => "\x1bOQ".into(),
        Key::F3 => "\x1bOR".into(),
        Key::F4 => "\x1bOS".into(),
        Key::F5 => "\x1b[15~".into(),
        Key::F6 => "\x1b[17~".into(),
        Key::F7 => "\x1b[18~".into(),
        Key::F8 => "\x1b[19~".into(),
        Key::F9 => "\x1b[20~".into(),
        Key::F10 => "\x1b[21~".into(),
        Key::F11 => "\x1b[23~".into(),
        Key::F12 => "\x1b[24~".into(),
        _ => return None,
    };
    Some(bytes.into_bytes())
}

fn terminal_color(color: vt100::Color, default: Color32, bold: bool) -> Color32 {
    match color {
        vt100::Color::Default => default,
        vt100::Color::Rgb(r, g, b) => Color32::from_rgb(r, g, b),
        vt100::Color::Idx(index) => {
            const PALETTE: [[u8; 3]; 16] = [
                [23, 29, 40],
                [229, 102, 115],
                [112, 210, 160],
                [235, 201, 117],
                [111, 166, 247],
                [183, 145, 237],
                [97, 205, 211],
                [203, 212, 226],
                [91, 105, 126],
                [255, 133, 144],
                [151, 238, 188],
                [255, 224, 150],
                [151, 190, 255],
                [212, 178, 255],
                [139, 233, 237],
                [243, 247, 253],
            ];
            let index = if bold && index < 8 { index + 8 } else { index };
            if index < 16 {
                let [r, g, b] = PALETTE[usize::from(index)];
                Color32::from_rgb(r, g, b)
            } else if index < 232 {
                let value = index - 16;
                let component = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
                Color32::from_rgb(
                    component(value / 36),
                    component((value / 6) % 6),
                    component(value % 6),
                )
            } else {
                let value = 8 + (index - 232) * 10;
                Color32::from_gray(value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{ChildKiller, ExitStatus};
    use std::sync::atomic::AtomicUsize;
    use std::time::Instant;

    #[derive(Clone, Debug)]
    struct RecordedChild {
        exited: bool,
        kills: Arc<AtomicUsize>,
        waits: Arc<AtomicUsize>,
    }

    impl ChildKiller for RecordedChild {
        fn kill(&mut self) -> std::io::Result<()> {
            self.kills.fetch_add(1, Ordering::SeqCst);
            self.exited = true;
            Ok(())
        }
        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }

    impl Child for RecordedChild {
        fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
            Ok(self.exited.then(|| ExitStatus::with_exit_code(0)))
        }
        fn wait(&mut self) -> std::io::Result<ExitStatus> {
            self.waits.fetch_add(1, Ordering::SeqCst);
            Ok(ExitStatus::with_exit_code(0))
        }
        fn process_id(&self) -> Option<u32> {
            None
        }
        #[cfg(windows)]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
            None
        }
    }

    #[test]
    fn closing_a_reaped_child_never_signals_its_reusable_pid() {
        for exited in [true, false] {
            let kills = Arc::new(AtomicUsize::new(0));
            let waits = Arc::new(AtomicUsize::new(0));
            reap_child(Box::new(RecordedChild {
                exited,
                kills: kills.clone(),
                waits: waits.clone(),
            }));
            let expected = usize::from(!exited);
            assert_eq!(kills.load(Ordering::SeqCst), expected);
            assert_eq!(waits.load(Ordering::SeqCst), expected);
        }
    }

    #[test]
    fn terminal_input_obeys_cursor_modes_and_control_keys() {
        assert_eq!(
            key_bytes(Key::ArrowUp, Modifiers::NONE, false).unwrap(),
            b"\x1b[A"
        );
        assert_eq!(
            key_bytes(Key::ArrowUp, Modifiers::NONE, true).unwrap(),
            b"\x1bOA"
        );
        assert_eq!(key_bytes(Key::C, Modifiers::CTRL, false).unwrap(), vec![3]);
        assert!(key_bytes(Key::C, Modifiers::MAC_CMD, false).is_none());
        assert_eq!(
            paste_bytes("hello\x1b[201~\r\n", true),
            b"\x1b[200~hello[201~\n\x1b[201~"
        );
    }

    #[test]
    fn local_pty_streams_output_and_reaps_process() {
        let mut command = CommandBuilder::new("/bin/sh");
        command.args([
            "-c",
            r#"printf '\033[32mPTY_READY\033[0m'; read answer; printf 'RECEIVED:%s' "$answer""#,
        ]);
        let mut session = TerminalSession::spawn("test".into(), command).unwrap();
        let start = Instant::now();
        while !session.screen_text().contains("PTY_READY")
            && start.elapsed() < Duration::from_secs(5)
        {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(session.screen_text().contains("PTY_READY"));
        session.resize(30, 90).unwrap();
        session.write(b"hello\n").unwrap();
        while !session.screen_text().contains("RECEIVED:hello")
            && start.elapsed() < Duration::from_secs(5)
        {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(session.screen_text().contains("RECEIVED:hello"));
        while !session.is_finished() && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(session.is_finished());
        assert!(session.write(&vec![0; MAX_INPUT_BYTES + 1]).is_err());
        session.close();
        assert!(session.write(b"x").is_err());
    }
}
