// tui.rs — interactive timeline browser.
//
// Two-pane layout:
//   left:  scrollable list of recent events (newest first), one row per event
//          with timestamp, kind (create/modify/delete), path, attribution.
//   right: unified diff for the currently selected event.
//
// Keys:
//   ↑ / k         move selection up
//   ↓ / j         move selection down
//   PageUp / PageDown — jump 10 rows
//   g / G         jump to first / last event
//   q / Esc / Ctrl-C — quit
//   r             refresh (re-query the timeline)
//
// This is the "wait, you can do that?" demo from USE_CASES.md. The selling
// point is the visual scrubber feel — arrow through every state of every
// file, watching the diff redraw in real time.

use anyhow::Result;
use chrono::{Local, TimeZone};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use similar::{ChangeTag, TextDiff};
use std::io;

use crate::store::{EventRow, Store};

pub fn run(store: &Store) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = main_loop(&mut terminal, store);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn main_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    store: &Store,
) -> Result<()> {
    let mut events = store.recent_events(500)?;
    let mut state = ListState::default();
    if !events.is_empty() {
        state.select(Some(0));
    }

    loop {
        terminal.draw(|f| draw(f, &events, &mut state, store))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(()),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                    (KeyCode::Char('r'), _) => {
                        events = store.recent_events(500)?;
                        if events.is_empty() {
                            state.select(None);
                        } else {
                            let cur = state.selected().unwrap_or(0).min(events.len() - 1);
                            state.select(Some(cur));
                        }
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        move_sel(&mut state, &events, 1)
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => move_sel(&mut state, &events, -1),
                    (KeyCode::PageDown, _) => move_sel(&mut state, &events, 10),
                    (KeyCode::PageUp, _) => move_sel(&mut state, &events, -10),
                    (KeyCode::Char('g'), _) => {
                        if !events.is_empty() {
                            state.select(Some(0));
                        }
                    }
                    (KeyCode::Char('G'), _) => {
                        if !events.is_empty() {
                            state.select(Some(events.len() - 1));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn move_sel(state: &mut ListState, events: &[EventRow], delta: i32) {
    if events.is_empty() {
        return;
    }
    let cur = state.selected().unwrap_or(0) as i32;
    let new = (cur + delta).clamp(0, events.len() as i32 - 1);
    state.select(Some(new as usize));
}

fn draw(f: &mut ratatui::Frame, events: &[EventRow], state: &mut ListState, store: &Store) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.size());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    draw_event_list(f, body[0], events, state);
    draw_diff_pane(f, body[1], events, state, store);
    draw_status_bar(f, chunks[1], events.len());
}

fn draw_event_list(f: &mut ratatui::Frame, area: Rect, events: &[EventRow], state: &mut ListState) {
    let items: Vec<ListItem> = events
        .iter()
        .map(|ev| {
            let when = Local
                .timestamp_nanos(ev.ts_ns)
                .format("%H:%M:%S")
                .to_string();
            let kind = match (&ev.before_hash, &ev.after_hash) {
                (None, Some(_)) => "create",
                (Some(_), Some(_)) => "modify",
                (Some(_), None) => "delete",
                (None, None) => "?",
            };
            let agent_color = match ev.attribution.as_str() {
                "claude-code" => Color::Cyan,
                "cursor" => Color::Magenta,
                "cline" => Color::Blue,
                "aider" | "codex" => Color::Yellow,
                "initial-scan" => Color::DarkGray,
                "agent-undo-restore" | "pre-restore" => Color::Green,
                _ => Color::White,
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{when}  "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{kind:<6} "), Style::default().fg(Color::White)),
                Span::styled(
                    format!("{:<28}", truncate(&ev.path, 28)),
                    Style::default().fg(Color::White),
                ),
                Span::styled(
                    format!(" [{}]", ev.attribution),
                    Style::default()
                        .fg(agent_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" timeline (j/k arrows, q quit, r refresh) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, state);
}

fn draw_diff_pane(
    f: &mut ratatui::Frame,
    area: Rect,
    events: &[EventRow],
    state: &ListState,
    store: &Store,
) {
    let title = " diff ";
    let block = Block::default().borders(Borders::ALL).title(title);

    let Some(idx) = state.selected() else {
        let p = Paragraph::new("(no events)").block(block);
        f.render_widget(p, area);
        return;
    };

    let Some(ev) = events.get(idx) else {
        f.render_widget(block, area);
        return;
    };

    let before = read_blob(store, ev.before_hash.as_deref());
    let after = read_blob(store, ev.after_hash.as_deref());

    let header = vec![
        Line::from(vec![
            Span::styled("event #", Style::default().fg(Color::DarkGray)),
            Span::styled(ev.id.to_string(), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled("path: ", Style::default().fg(Color::DarkGray)),
            Span::styled(ev.path.clone(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("agent: ", Style::default().fg(Color::DarkGray)),
            Span::styled(ev.attribution.clone(), Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ev.session_id.clone().unwrap_or_else(|| "—".into()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
    ];

    let mut lines: Vec<Line> = header;
    let diff = TextDiff::from_lines(before.as_str(), after.as_str());
    for change in diff.iter_all_changes() {
        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", Color::Red),
            ChangeTag::Insert => ("+", Color::Green),
            ChangeTag::Equal => (" ", Color::DarkGray),
        };
        let text = change.value().trim_end_matches('\n').to_string();
        lines.push(Line::from(vec![
            Span::styled(sign.to_string(), Style::default().fg(color)),
            Span::styled(text, Style::default().fg(color)),
        ]));
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_status_bar(f: &mut ratatui::Frame, area: Rect, total: usize) {
    let p = Paragraph::new(format!(
        "agent-undo tui — {total} events  |  ↑↓ navigate  PgUp/PgDn jump  g/G first/last  r refresh  q quit"
    ))
    .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(p, area);
}

fn read_blob(store: &Store, hash: Option<&str>) -> String {
    match hash {
        Some(h) => store
            .read_blob(h)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default(),
        None => String::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let cut = max.saturating_sub(1);
        let mut out: String = s.chars().take(cut).collect();
        out.push('…');
        out
    }
}
