//! The ratatui frontend.
//!
//! The renderer is stateless beyond cursor concerns: every frame draws
//! whatever [`View`] snapshot the owner last published, plus a small
//! [`UiState`] (input buffer, selected channel, the peer-id dialog). Input
//! arrives from a dedicated OS thread that polls crossterm and forwards
//! events over a channel, so the async render loop never blocks on the
//! terminal.
//!
//! Layout: channels left, the selected channel's causally ordered messages
//! center, the presence roster right; a header that always shows our own
//! endpoint id (ready to share), and an input line at the bottom. The app
//! boots into a centered "connect to a peer" dialog (skipped when `--peer`
//! was given); `Ctrl-P` reopens it any time. Messages that arrived out of
//! causal order and landed mid-list render highlighted until their flash
//! expires.

use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context as _;
use iroh::EndpointId;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, poll, read};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use tokio::sync::{mpsc, watch};

use crate::entry::{Millis, PeerId};
use crate::owner::Command;
use crate::timers;
use crate::view::View;

/// What an input event asked of the app.
#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Quit,
}

/// Renderer-local state: everything that is about *this* terminal rather
/// than the replicated world.
struct UiState {
    /// The chat input line.
    input: String,
    /// Index into `view.channels` of the channel on screen.
    selected: usize,
    /// The peer-id dialog: `Some(buffer)` while open.
    dialog: Option<String>,
    /// A `/new` was sent; select the channel once it appears.
    pending_select: Option<String>,
    /// A transient error line (e.g. an unparseable peer id).
    flash: Option<String>,
}

/// Forward crossterm events into the async world. The thread exits when the
/// receiver is dropped.
pub fn spawn_input_thread(tx: mpsc::Sender<Event>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            match poll(timers::UI_TICK) {
                Ok(true) => match read() {
                    Ok(event) => {
                        if tx.blocking_send(event).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                },
                Ok(false) => {
                    if tx.is_closed() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    })
}

/// Drive the terminal until the user quits (or the owner goes away).
pub async fn run(
    terminal: &mut DefaultTerminal,
    cmd: mpsc::Sender<Command>,
    mut view_rx: watch::Receiver<Arc<View>>,
    mut input_rx: mpsc::Receiver<Event>,
    open_dialog: bool,
) -> anyhow::Result<()> {
    let mut ui = UiState {
        input: String::new(),
        selected: 0,
        dialog: open_dialog.then(String::new),
        pending_select: None,
        flash: None,
    };
    let mut tick = tokio::time::interval(timers::UI_TICK);
    loop {
        {
            let view = view_rx.borrow().clone();
            reconcile(&mut ui, &view);
            terminal
                .draw(|frame| draw(frame, &view, &ui))
                .context("drawing the frame")?;
        }
        tokio::select! {
            changed = view_rx.changed() => {
                if changed.is_err() {
                    return Ok(()); // the owner is gone; we are shutting down
                }
            }
            event = input_rx.recv() => match event {
                Some(event) => {
                    if handle(&mut ui, event, &cmd, &view_rx.borrow().clone()).await? == Flow::Quit {
                        return Ok(());
                    }
                }
                None => return Ok(()),
            },
            // Ages out highlight flashes and refreshes "last seen" ages.
            _ = tick.tick() => {}
        }
    }
}

/// Clamp renderer state against the current view (channels can appear,
/// vanish, or reorder under us).
fn reconcile(ui: &mut UiState, view: &View) {
    if let Some(name) = &ui.pending_select
        && let Some(index) = view.channels.iter().position(|c| &c.name == name)
    {
        ui.selected = index;
        ui.pending_select = None;
    }
    if ui.selected >= view.channels.len() {
        ui.selected = view.channels.len().saturating_sub(1);
    }
}

/// Apply one input event, sending commands to the owner as needed.
async fn handle(
    ui: &mut UiState,
    event: Event,
    cmd: &mpsc::Sender<Command>,
    view: &View,
) -> anyhow::Result<Flow> {
    let Event::Key(key) = &event else {
        // Pasted text lands in whichever buffer is active.
        if let Event::Paste(text) = &event {
            match &mut ui.dialog {
                Some(buffer) => buffer.push_str(text),
                None => ui.input.push_str(text),
            }
        }
        return Ok(Flow::Continue);
    };
    if key.kind == KeyEventKind::Release {
        return Ok(Flow::Continue);
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => return Ok(Flow::Quit),
            KeyCode::Char('p') => {
                ui.dialog = Some(String::new());
                ui.flash = None;
            }
            _ => {}
        }
        return Ok(Flow::Continue);
    }

    if let Some(buffer) = &mut ui.dialog {
        match key.code {
            KeyCode::Esc => {
                ui.dialog = None;
                ui.flash = None;
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => buffer.push(c),
            KeyCode::Enter => match buffer.trim().parse::<EndpointId>() {
                Ok(peer) => {
                    cmd.send(Command::AddPeer {
                        peer: *peer.as_bytes(),
                    })
                    .await
                    .context("owner gone")?;
                    ui.dialog = None;
                    ui.flash = None;
                }
                Err(e) => ui.flash = Some(format!("not a peer id: {e}")),
            },
            _ => {}
        }
        return Ok(Flow::Continue);
    }

    match key.code {
        KeyCode::Esc => return Ok(Flow::Quit),
        KeyCode::Tab => {
            if !view.channels.is_empty() {
                ui.selected = (ui.selected + 1) % view.channels.len();
            }
        }
        KeyCode::BackTab => {
            if !view.channels.is_empty() {
                ui.selected = (ui.selected + view.channels.len() - 1) % view.channels.len();
            }
        }
        KeyCode::Backspace => {
            ui.input.pop();
        }
        KeyCode::Char(c) => ui.input.push(c),
        KeyCode::Enter => {
            let line = std::mem::take(&mut ui.input);
            let line = line.trim();
            if line.is_empty() {
                return Ok(Flow::Continue);
            }
            ui.flash = None;
            if let Some(name) = line.strip_prefix("/new ") {
                let name = name.trim().trim_start_matches('#').to_string();
                if name.is_empty() {
                    ui.flash = Some("usage: /new <channel>".into());
                } else {
                    ui.pending_select = Some(name.clone());
                    cmd.send(Command::CreateChannel { name })
                        .await
                        .context("owner gone")?;
                }
            } else if line.starts_with('/') {
                ui.flash = Some(format!("unknown command: {line}"));
            } else if let Some(channel) = view.channels.get(ui.selected) {
                cmd.send(Command::SendChat {
                    channel: channel.name.clone(),
                    body: line.to_string(),
                })
                .await
                .context("owner gone")?;
            }
        }
        _ => {}
    }
    Ok(Flow::Continue)
}

fn draw(frame: &mut Frame<'_>, view: &View, ui: &UiState) {
    let [header, body, input, footer] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(frame.area());
    let [channels, messages, roster] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),
            Constraint::Min(20),
            Constraint::Length(26),
        ])
        .areas(body);

    draw_header(frame, header, view);
    draw_channels(frame, channels, view, ui);
    draw_messages(frame, messages, view, ui);
    draw_roster(frame, roster, view);
    draw_input(frame, input, view, ui);
    draw_footer(frame, footer, ui);
    if let Some(buffer) = &ui.dialog {
        draw_dialog(frame, buffer, ui.flash.as_deref());
    }
}

fn draw_header(frame: &mut Frame<'_>, area: Rect, view: &View) {
    let stats = view.stats;
    let mut status = vec![
        Span::styled(" rumormill ", Style::new().bold().fg(Color::Magenta)),
        Span::raw(format!("· {} ", view.name)),
        Span::styled(
            format!("· net {} ", view.network),
            Style::new().fg(Color::DarkGray),
        ),
        Span::raw(format!(
            "· {} live · sync {}✓ {}✗ ",
            stats.live_entries, stats.sessions_ok, stats.sessions_failed
        )),
    ];
    if let Some(notice) = &view.merged_notice {
        status.push(Span::styled(
            format!("· {notice} "),
            Style::new().fg(Color::Yellow).bold(),
        ));
    }
    let id = Line::from(vec![
        Span::styled(" you: ", Style::new().fg(Color::DarkGray)),
        Span::styled(view.me_display.clone(), Style::new().fg(Color::Cyan)),
        Span::styled("  (share this id)", Style::new().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::from(status), id]), area);
}

fn draw_channels(frame: &mut Frame<'_>, area: Rect, view: &View, ui: &UiState) {
    let items: Vec<ListItem> = view
        .channels
        .iter()
        .enumerate()
        .map(|(i, channel)| {
            let line = format!("#{} ({})", channel.name, channel.messages.len());
            let style = if i == ui.selected {
                Style::new().bold().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::new()
            };
            ListItem::new(Line::styled(line, style))
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("channels")),
        area,
    );
}

fn draw_messages(frame: &mut Frame<'_>, area: Rect, view: &View, ui: &UiState) {
    let now = Instant::now();
    let title = view
        .channels
        .get(ui.selected)
        .map(|c| format!("#{}", c.name))
        .unwrap_or_else(|| "no channel".into());
    let visible = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = view
        .channels
        .get(ui.selected)
        .map(|channel| {
            let skip = channel.messages.len().saturating_sub(visible);
            channel.messages[skip..]
                .iter()
                .map(|m| {
                    let highlighted = m.highlight_until.is_some_and(|until| until > now);
                    let mut spans = vec![Span::styled(
                        format!("{} ", fmt_clock(m.at)),
                        Style::new().fg(Color::DarkGray),
                    )];
                    match m.author {
                        Some(author) => {
                            let me = author == view.me;
                            let style = Style::new().fg(author_color(&author));
                            let style = if me { style.bold() } else { style };
                            spans.push(Span::styled(format!("{}: ", m.author_name), style));
                            spans.push(Span::raw(m.body.clone()));
                        }
                        None => spans.push(Span::styled(
                            m.body.clone(),
                            Style::new().fg(Color::DarkGray).italic(),
                        )),
                    }
                    let line = Line::from(spans);
                    if highlighted {
                        // Landed mid-list: delivered out of causal order.
                        line.style(Style::new().bg(Color::Rgb(64, 64, 0)))
                    } else {
                        line
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn draw_roster(frame: &mut Frame<'_>, area: Rect, view: &View) {
    let now = now_ms();
    let items: Vec<ListItem> = view
        .roster
        .iter()
        .map(|peer| {
            let ago = now.saturating_sub(peer.last_seen) / 1000;
            let quiet = ago > 2 * timers::HEARTBEAT_INTERVAL.as_secs();
            let style = if peer.peer == view.me {
                Style::new().bold()
            } else if quiet {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new()
            };
            ListItem::new(Line::styled(format!("{} ({ago}s)", peer.name), style))
        })
        .collect();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("peers ({})", view.roster.len())),
        ),
        area,
    );
}

fn draw_input(frame: &mut Frame<'_>, area: Rect, view: &View, ui: &UiState) {
    let target = view
        .channels
        .get(ui.selected)
        .map(|c| format!("#{}", c.name))
        .unwrap_or_else(|| "—".into());
    let text = match &ui.flash {
        Some(flash) if ui.dialog.is_none() => {
            Line::styled(flash.clone(), Style::new().fg(Color::Red))
        }
        _ => Line::from(vec![
            Span::styled(format!("{target} › "), Style::new().fg(Color::Cyan)),
            Span::raw(ui.input.clone()),
            Span::styled("▏", Style::new().add_modifier(Modifier::SLOW_BLINK)),
        ]),
    };
    frame.render_widget(
        Paragraph::new(text).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, ui: &UiState) {
    let hints = if ui.dialog.is_some() {
        " paste a peer id · Enter connect · Esc cancel"
    } else {
        " Enter send · Tab channel · /new <name> · ^P add peer · Esc quit"
    };
    frame.render_widget(
        Paragraph::new(Line::styled(hints, Style::new().fg(Color::DarkGray))),
        area,
    );
}

fn draw_dialog(frame: &mut Frame<'_>, buffer: &str, flash: Option<&str>) {
    let area = centered(frame.area(), 64, 7);
    frame.render_widget(Clear, area);
    let lines = vec![
        Line::raw(""),
        Line::from(vec![
            Span::raw("  peer id: "),
            Span::styled(buffer.to_string(), Style::new().fg(Color::Cyan)),
            Span::styled("▏", Style::new().add_modifier(Modifier::SLOW_BLINK)),
        ]),
        Line::raw(""),
        match flash {
            Some(flash) => Line::styled(format!("  {flash}"), Style::new().fg(Color::Red)),
            None => Line::styled(
                "  gossip spreads from a single contact",
                Style::new().fg(Color::DarkGray),
            ),
        },
    ];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" connect to a peer ")
                .border_style(Style::new().fg(Color::Cyan)),
        ),
        area,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// A stable per-author color drawn from the peer id.
fn author_color(peer: &PeerId) -> Color {
    const PALETTE: [Color; 6] = [
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Red,
    ];
    PALETTE[peer[0] as usize % PALETTE.len()]
}

/// Epoch milliseconds as a UTC `HH:MM:SS` wall-clock label.
fn fmt_clock(at: Millis) -> String {
    let seconds = at / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn now_ms() -> Millis {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is past the epoch")
        .as_millis() as Millis
}
