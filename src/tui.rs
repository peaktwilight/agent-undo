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

const EVENT_LIMIT: usize = 500;
const DIFF_SCROLL_STEP: u16 = 6;

struct TuiState {
    events: Vec<EventRow>,
    list: ListState,
    agent_filter: Option<String>,
    path_filter: Option<String>,
    diff_scroll: u16,
}

impl TuiState {
    fn load(store: &Store) -> Result<Self> {
        let mut state = Self {
            events: Vec::new(),
            list: ListState::default(),
            agent_filter: None,
            path_filter: None,
            diff_scroll: 0,
        };
        state.reload(store)?;
        Ok(state)
    }

    fn reload(&mut self, store: &Store) -> Result<()> {
        let selected_id = self.selected_event().map(|ev| ev.id);
        self.events = store.filtered_events(
            self.agent_filter.as_deref(),
            self.path_filter.as_deref(),
            None,
            EVENT_LIMIT,
        )?;
        self.restore_selection(selected_id);
        self.diff_scroll = 0;
        Ok(())
    }

    fn selected_event(&self) -> Option<&EventRow> {
        self.list.selected().and_then(|idx| self.events.get(idx))
    }

    fn restore_selection(&mut self, selected_id: Option<i64>) {
        let idx = if self.events.is_empty() {
            None
        } else if let Some(id) = selected_id {
            self.events.iter().position(|ev| ev.id == id).or(Some(0))
        } else {
            Some(0)
        };
        self.list.select(idx);
    }

    fn set_selected(&mut self, idx: Option<usize>) {
        self.list.select(idx);
        self.diff_scroll = 0;
    }

    fn move_selection(&mut self, delta: i32) {
        if self.events.is_empty() {
            return;
        }
        let cur = self.list.selected().unwrap_or(0) as i32;
        let new = (cur + delta).clamp(0, self.events.len() as i32 - 1);
        self.set_selected(Some(new as usize));
    }

    fn jump_first(&mut self) {
        if !self.events.is_empty() {
            self.set_selected(Some(0));
        }
    }

    fn jump_last(&mut self) {
        if !self.events.is_empty() {
            self.set_selected(Some(self.events.len() - 1));
        }
    }

    fn toggle_selected_agent_filter(&mut self, store: &Store) -> Result<()> {
        let Some(agent) = self.selected_event().map(|ev| ev.attribution.clone()) else {
            return Ok(());
        };
        if self.agent_filter.as_deref() == Some(agent.as_str()) {
            self.agent_filter = None;
        } else {
            self.agent_filter = Some(agent);
        }
        self.reload(store)
    }

    fn toggle_selected_path_filter(&mut self, store: &Store) -> Result<()> {
        let Some(path) = self.selected_event().map(|ev| ev.path.clone()) else {
            return Ok(());
        };
        if self.path_filter.as_deref() == Some(path.as_str()) {
            self.path_filter = None;
        } else {
            self.path_filter = Some(path);
        }
        self.reload(store)
    }

    fn clear_filters(&mut self, store: &Store) -> Result<()> {
        if self.agent_filter.is_none() && self.path_filter.is_none() {
            return Ok(());
        }
        self.agent_filter = None;
        self.path_filter = None;
        self.reload(store)
    }

    fn scroll_diff(&mut self, delta: i16) {
        self.diff_scroll = self.diff_scroll.saturating_add_signed(delta);
    }
}

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
    let mut state = TuiState::load(store)?;

    loop {
        terminal.draw(|f| draw(f, &state, store))?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(()),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => return Ok(()),
                    (KeyCode::Char('r'), _) => {
                        state.reload(store)?;
                    }
                    (KeyCode::Char('a'), _) => state.toggle_selected_agent_filter(store)?,
                    (KeyCode::Char('f'), _) => state.toggle_selected_path_filter(store)?,
                    (KeyCode::Char('u'), _) => state.clear_filters(store)?,
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => state.move_selection(1),
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => state.move_selection(-1),
                    (KeyCode::PageDown, _) => state.move_selection(10),
                    (KeyCode::PageUp, _) => state.move_selection(-10),
                    (KeyCode::Char('g'), _) => state.jump_first(),
                    (KeyCode::Char('G'), _) => state.jump_last(),
                    (KeyCode::Char(']'), _) => state.scroll_diff(DIFF_SCROLL_STEP as i16),
                    (KeyCode::Char('['), _) => state.scroll_diff(-(DIFF_SCROLL_STEP as i16)),
                    _ => {}
                }
            }
        }
    }
}

fn draw(f: &mut ratatui::Frame, state: &TuiState, store: &Store) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(f.size());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    draw_event_list(f, body[0], state);
    draw_diff_pane(f, body[1], state, store);
    draw_status_bar(f, chunks[1], state);
}

fn draw_event_list(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let items: Vec<ListItem> = state
        .events
        .iter()
        .map(|ev| {
            let when = Local
                .timestamp_nanos(ev.ts_ns)
                .format("%H:%M:%S")
                .to_string();
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
                Span::styled(
                    format!("{:<6} ", event_kind(ev)),
                    Style::default().fg(Color::White),
                ),
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

    let title = format!(" timeline ({}) ", filters_label(state));

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = state.list.clone();
    f.render_stateful_widget(list, area, &mut list_state);
}

fn draw_diff_pane(f: &mut ratatui::Frame, area: Rect, state: &TuiState, store: &Store) {
    let title = format!(" diff (scroll {} ) ", state.diff_scroll);
    let block = Block::default().borders(Borders::ALL).title(title);

    let Some(idx) = state.list.selected() else {
        let p = Paragraph::new("(no events)").block(block);
        f.render_widget(p, area);
        return;
    };

    let Some(ev) = state.events.get(idx) else {
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
            Span::styled("kind: ", Style::default().fg(Color::DarkGray)),
            Span::styled(event_kind(ev), Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled("session: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                ev.session_id.clone().unwrap_or_else(|| "—".into()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("sizes: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                size_label(ev.size_before),
                Style::default().fg(Color::White),
            ),
            Span::raw(" -> "),
            Span::styled(size_label(ev.size_after), Style::default().fg(Color::White)),
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
        .scroll((state.diff_scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_status_bar(f: &mut ratatui::Frame, area: Rect, state: &TuiState) {
    let p = Paragraph::new(format!(
        "agent-undo tui | {} events | {} | ↑↓ move PgUp/PgDn jump g/G ends [ ] diff a agent f file u clear r refresh q quit",
        state.events.len(),
        filters_label(state)
    ))
    .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(p, area);
}

fn event_kind(ev: &EventRow) -> &'static str {
    match (&ev.before_hash, &ev.after_hash) {
        (None, Some(_)) => "create",
        (Some(_), Some(_)) => "modify",
        (Some(_), None) => "delete",
        (None, None) => "?",
    }
}

fn filters_label(state: &TuiState) -> String {
    let mut parts = Vec::new();
    if let Some(agent) = &state.agent_filter {
        parts.push(format!("agent:{}", truncate(agent, 16)));
    }
    if let Some(path) = &state.path_filter {
        parts.push(format!("file:{}", truncate(path, 24)));
    }
    if parts.is_empty() {
        "all events".to_string()
    } else {
        parts.join("  ")
    }
}

fn size_label(size: Option<i64>) -> String {
    match size {
        Some(size) => format!("{size}b"),
        None => "—".to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::{event_kind, size_label, TuiState};
    use crate::store::EventRow;
    use ratatui::widgets::ListState;

    fn event(id: i64, path: &str, agent: &str) -> EventRow {
        EventRow {
            id,
            ts_ns: id,
            path: path.to_string(),
            before_hash: Some(format!("before-{id}")),
            after_hash: Some(format!("after-{id}")),
            size_before: Some(10),
            size_after: Some(11),
            attribution: agent.to_string(),
            session_id: Some(format!("session-{id}")),
        }
    }

    #[test]
    fn restore_selection_prefers_previous_event_id() {
        let mut state = TuiState {
            events: vec![event(1, "a.rs", "claude-code"), event(2, "b.rs", "cursor")],
            list: ListState::default(),
            agent_filter: None,
            path_filter: None,
            diff_scroll: 0,
        };

        state.restore_selection(Some(2));
        assert_eq!(state.list.selected(), Some(1));
    }

    #[test]
    fn restore_selection_falls_back_to_first_event() {
        let mut state = TuiState {
            events: vec![event(10, "a.rs", "claude-code")],
            list: ListState::default(),
            agent_filter: None,
            path_filter: None,
            diff_scroll: 0,
        };

        state.restore_selection(Some(999));
        assert_eq!(state.list.selected(), Some(0));
    }

    #[test]
    fn event_kind_reports_modify_rows() {
        assert_eq!(event_kind(&event(1, "a.rs", "claude-code")), "modify");
    }

    #[test]
    fn size_label_formats_unknown_values() {
        assert_eq!(size_label(None), "—");
        assert_eq!(size_label(Some(42)), "42b");
    }
}
