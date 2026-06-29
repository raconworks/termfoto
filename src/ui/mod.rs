pub mod browser;
pub mod layout;
pub mod preview;
pub mod search;

use crate::app::{App, AppState, DirectoryContextEntry, DirectoryContextKind, LOGO_HEIGHT};
use crate::scanner::ImageEntry;
use crate::ui::browser::BrowserView;
use crate::ui::preview::PreviewView;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};
use std::time::{SystemTime, UNIX_EPOCH};

const LOGO_LINES: [&str; LOGO_HEIGHT as usize] = [
    "▀█▀ █▀▀ █▀█ █▀▄▀█ █▀▀ █▀█ ▀█▀ █▀█",
    " █  █▀  █▀▄ █ ▀ █ █▀  █▄█  █  █▄█",
    " ▀  ▀▀▀ ▀ ▀ ▀   ▀ ▀   ▀▀▀  ▀  ▀▀▀",
];

const LOGO_COLORS: [Color; LOGO_HEIGHT as usize] = [
    Color::Rgb(255, 0, 0),
    Color::Rgb(0, 255, 0),
    Color::Rgb(127, 0, 255),
];

pub fn render_logo(area: Rect, buf: &mut Buffer) {
    let max_w = LOGO_LINES
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    let logo_w = max_w.min(area.width as usize);
    let offset_x = area.x + area.width.saturating_sub(logo_w as u16);

    for (i, line) in LOGO_LINES.iter().enumerate() {
        if i as u16 >= area.height {
            break;
        }
        let trimmed: String = line.chars().take(logo_w).collect();
        let style = Style::default().fg(LOGO_COLORS[i]);
        buf.set_span(
            offset_x,
            area.y + i as u16,
            &Span::styled(trimmed, style),
            logo_w as u16,
        );
    }
}

pub fn draw(frame: &mut Frame, app: &mut App, cell_w: u16, cell_h: u16) {
    let area = frame.area();
    match app.state {
        AppState::Browser => {
            frame.render_widget(
                BrowserView {
                    app,
                    cell_w,
                    cell_h,
                },
                area,
            );
        }
        AppState::Fullscreen => {
            frame.render_widget(PreviewView { app }, area);
        }
    }
}

pub fn render_panel(area: Rect, title: &str, buf: &mut Buffer) -> Rect {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    block.render(area, buf);
    inner
}

pub fn render_directory_context(
    area: Rect,
    entries: &[DirectoryContextEntry],
    empty_text: &str,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    if entries.is_empty() {
        Paragraph::new(Span::styled(
            truncate_to_width(empty_text, area.width),
            Style::default().fg(Color::DarkGray),
        ))
        .render(area, buf);
        return;
    }

    let lines: Vec<Line> = entries
        .iter()
        .take(area.height as usize)
        .map(|entry| {
            let suffix = match entry.kind {
                DirectoryContextKind::Directory => "/",
                DirectoryContextKind::File => "",
            };
            let marker = if entry.is_current { "> " } else { "  " };
            let text =
                truncate_to_width(&format!("{}{}{}", marker, entry.name, suffix), area.width);
            let style = if entry.is_current {
                Style::default().fg(Color::Cyan)
            } else if entry.kind == DirectoryContextKind::Directory {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::from(Span::styled(text, style))
        })
        .collect();

    Paragraph::new(lines).render(area, buf);
}

pub fn render_info_panel(
    area: Rect,
    entry: Option<&ImageEntry>,
    dims: Option<(u32, u32)>,
    app: &App,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let Some(entry) = entry else {
        return;
    };

    let lang = &app.lang;
    let ext = entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("?")
        .to_uppercase();

    let mut lines = Vec::new();
    lines.push(format!("{}: {}", lang.label_file(), entry.filename));
    if let Some((w, h)) = dims {
        lines.push(format!("{}: {}x{}", lang.label_dims(), w, h));
    }
    lines.push(format!(
        "{}: {}",
        lang.label_size(),
        format_size(entry.file_size)
    ));
    lines.push(format!("{}: {}", lang.label_type(), ext));
    if let Ok(metadata) = std::fs::metadata(&entry.path) {
        if let Ok(modified) = metadata.modified() {
            lines.push(format!(
                "{}: {}",
                lang.label_modified(),
                format_system_time(modified)
            ));
        }
        if let Ok(created) = metadata.created() {
            lines.push(format!(
                "{}: {}",
                lang.label_created(),
                format_system_time(created)
            ));
        }
    }
    lines.push(format!(
        "{}: {}",
        lang.label_path(),
        entry.path.to_string_lossy()
    ));

    let text_lines: Vec<Line> = lines
        .into_iter()
        .take(area.height as usize)
        .map(|line| {
            Line::from(Span::styled(
                truncate_to_width(&line, area.width),
                Style::default().fg(Color::White),
            ))
        })
        .collect();
    Paragraph::new(text_lines).render(area, buf);
}

pub fn render_prompt_lines(area: Rect, lines: &[String], buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    render_prompt_base(area, buf);
    let prompt_lines: Vec<Line> = lines
        .iter()
        .take(area.height as usize)
        .map(|line| {
            Line::from(Span::styled(
                truncate_to_width(line, area.width),
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ))
        })
        .collect();
    Paragraph::new(prompt_lines).render(area, buf);
}

pub fn render_prompt_base(area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buf.set_style(area, Style::default().bg(Color::DarkGray));
    render_logo(area, buf);
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn format_system_time(time: SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return "before 1970-01-01".to_string();
    };
    let total_seconds = duration.as_secs();
    let days = (total_seconds / 86_400) as i64;
    let seconds_of_day = total_seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hour, minute, second
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

fn truncate_to_width(text: &str, width: u16) -> String {
    let max = width as usize;
    if max == 0 {
        return String::new();
    }
    if text.chars().count() <= max {
        return text.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }
    let mut truncated: String = text.chars().take(max - 1).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppStart, AppState, LoadRequest, LoadResult};
    use crate::lang::Lang;
    use crate::scanner::ImageEntry;
    use ratatui_image::picker::Picker;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn compact_logo_uses_three_rows() {
        assert_eq!(LOGO_HEIGHT, 3);
        assert_eq!(LOGO_LINES.len(), LOGO_HEIGHT as usize);
        assert_eq!(LOGO_COLORS.len(), LOGO_HEIGHT as usize);
    }

    #[test]
    #[test]
    fn panels_leave_prompt_on_bottom_three_rows() {
        let areas = crate::ui::layout::three_panel_areas(Rect::new(0, 0, 100, 30));

        assert_eq!(areas.prompt.y, 27);
        assert_eq!(areas.prompt.height, crate::ui::layout::PROMPT_HEIGHT);
    }

    #[test]
    fn prompt_base_draws_logo_on_right() {
        let area = Rect::new(0, 0, 80, crate::ui::layout::PROMPT_HEIGHT);
        let mut buf = Buffer::empty(area);

        render_prompt_base(area, &mut buf);

        assert_eq!(buf.cell((area.width - 1, 0)).unwrap().symbol(), "█");
    }

    #[test]
    fn system_time_formats_as_utc_datetime() {
        assert_eq!(
            format_system_time(UNIX_EPOCH + Duration::from_secs(86_400)),
            "1970-01-02 00:00:00 UTC"
        );
    }

    #[test]
    fn info_panel_lists_modified_time() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sample.png");
        std::fs::write(&path, b"sample").unwrap();
        let entry = ImageEntry {
            path: path.clone(),
            filename: "sample.png".to_string(),
            file_size: 6,
        };
        let (tx, _rx) = std::sync::mpsc::channel::<LoadRequest>();
        let (_tx2, rx2) = std::sync::mpsc::channel::<LoadResult>();
        let app = App::new(
            AppStart {
                images: vec![entry],
                image_dir: PathBuf::from(dir.path()),
                state: AppState::Browser,
                selected: 0,
            },
            tx,
            rx2,
            Lang::En,
            Picker::halfblocks(),
        );
        let area = Rect::new(0, 0, 80, 8);
        let mut buf = Buffer::empty(area);

        render_info_panel(area, app.images.first(), None, &app, &mut buf);

        let text: String = buf.content().iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("Modified"));
    }
}
