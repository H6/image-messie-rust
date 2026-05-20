use std::io;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Terminal,
};

use crate::models::Stats;

/// Display a final statistics panel using ratatui.
/// Blocks until the user presses `q`, `Enter`, or `Escape`.
pub fn show_stats(stats: &Stats, dry_run: bool) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3), // title
                    Constraint::Length(if dry_run { 3 } else { 0 }), // dry-run notice
                    Constraint::Length(5), // stats table
                    Constraint::Length(2), // hint
                    Constraint::Min(0),
                ])
                .split(area);

            // ── Title ────────────────────────────────────────────────────────
            let title = Block::default()
                .title(" Image Messie — Results ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
            frame.render_widget(title, chunks[0]);

            // ── Dry-run notice ───────────────────────────────────────────────
            if dry_run {
                let notice = Block::default()
                    .title(" ⚠  DRY RUN — no files were copied ")
                    .title_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow));
                frame.render_widget(notice, chunks[1]);
            }

            // ── Stats table ──────────────────────────────────────────────────
            let header_cells = ["Total Images", "Total Non-Images", "Total Size (MB)"]
                .iter()
                .map(|h| {
                    Cell::from(*h).style(
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    )
                });
            let header = Row::new(header_cells).height(1).bottom_margin(1);

            let size_mb = format!("{:.2}", stats.total_size_mb());
            let data_cells = [
                Cell::from(stats.total_images.to_string())
                    .style(Style::default().fg(Color::Green)),
                Cell::from(stats.total_non_images.to_string())
                    .style(Style::default().fg(Color::Red)),
                Cell::from(size_mb).style(Style::default().fg(Color::White)),
            ];
            let row = Row::new(data_cells).height(1);

            let table = Table::new(
                vec![row],
                [
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            frame.render_widget(table, chunks[2]);

            // ── Hint ─────────────────────────────────────────────────────────
            let hint = Line::from(vec![Span::styled(
                "Press q or Enter to exit",
                Style::default().fg(Color::DarkGray),
            )]);
            frame.render_widget(
                ratatui::widgets::Paragraph::new(hint),
                chunks[3],
            );
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Enter | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
