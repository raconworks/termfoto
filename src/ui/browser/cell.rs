use crate::app::ImageCacheKey;
use lru::LruCache;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Widget},
};
use ratatui_image::{protocol::Protocol, Image};

/// Truncate filename to fit cell width, appending "…" if needed.
fn truncate_filename(name: &str, max_width: u16) -> String {
    let max = max_width as usize;
    if name.chars().count() <= max {
        name.to_string()
    } else {
        let mut s: String = name.chars().take(max.saturating_sub(1)).collect();
        s.push('…');
        s
    }
}

pub(super) struct CellMeta<'a> {
    pub(super) filename: &'a str,
    pub(super) selected: bool,
    pub(super) search_match: bool,
    pub(super) search_query: Option<&'a str>,
    pub(super) favorite: bool,
    pub(super) favorite_label: Option<&'a str>,
    pub(super) cache_key: Option<ImageCacheKey>,
}

pub(super) fn render_browser_cell(
    area: Rect,
    buf: &mut Buffer,
    meta: &CellMeta,
    cache: &LruCache<ImageCacheKey, Protocol>,
) {
    let border_style = if meta.selected {
        // Both selected and search match: bright yellow
        if meta.search_match {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Cyan)
        }
    } else if meta.search_match {
        // Search match but not selected: dim yellow
        Style::default().fg(Color::Rgb(128, 128, 0))
    } else if meta.favorite {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .render(area, buf);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    // Truncate filename to fit cell width
    let name = truncate_filename(meta.filename, inner.width);

    let badge_height = meta.favorite_label.is_some() as u16;
    let name_height = 1u16;
    let badge_area = Rect {
        y: inner.y,
        height: badge_height,
        ..inner
    };
    let thumb_area = Rect {
        y: inner.y + badge_height,
        height: inner.height.saturating_sub(name_height + badge_height),
        ..inner
    };
    let name_area = Rect {
        y: inner.y + badge_height + thumb_area.height,
        height: name_height,
        ..inner
    };

    if let Some(label) = meta.favorite_label {
        let label = truncate_filename(label, badge_area.width);
        let style = if meta.selected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Magenta)
        };
        let label_width = label.chars().count() as u16;
        let label_x = badge_area.x + (badge_area.width.saturating_sub(label_width)) / 2;
        buf.set_span(
            label_x,
            badge_area.y,
            &Span::styled(label, style),
            label_width,
        );
    }

    // Render chafa thumbnail centered
    if let Some(proto) = meta.cache_key.as_ref().and_then(|key| cache.peek(key)) {
        let proto_size = proto.size();
        let offset_x = thumb_area.width.saturating_sub(proto_size.width) / 2;
        let offset_y = thumb_area.height.saturating_sub(proto_size.height) / 2;
        let centered = Rect {
            x: thumb_area.x + offset_x,
            y: thumb_area.y + offset_y,
            width: proto_size.width.min(thumb_area.width),
            height: proto_size.height.min(thumb_area.height),
        };
        Image::new(proto).allow_clipping(true).render(centered, buf);
    }

    // Render filename with match highlighting if in search mode
    let matched_char_style = if meta.selected || meta.search_match {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Rgb(200, 200, 0))
    };
    let normal_style = if meta.selected {
        Style::default().fg(Color::Cyan)
    } else if meta.search_match {
        Style::default().fg(Color::Rgb(200, 200, 200))
    } else if meta.favorite {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::White)
    };

    if let Some(query) = meta.search_query {
        if !query.is_empty() {
            render_filename_with_highlight(
                name_area,
                buf,
                &name,
                query,
                matched_char_style,
                normal_style,
            );
            return;
        }
    }

    // No search / empty query: centered single-span filename
    let span: Span;
    if meta.selected {
        span = Span::styled(name.clone(), Style::default().fg(Color::Cyan));
    } else if meta.search_match {
        span = Span::styled(name.clone(), Style::default().fg(Color::Rgb(200, 200, 200)));
    } else if meta.favorite {
        span = Span::styled(name.clone(), Style::default().fg(Color::Magenta));
    } else {
        span = Span::styled(name.clone(), Style::default().fg(Color::White));
    }
    let name_width = name.chars().count() as u16;
    let name_x = name_area.x + (name_area.width.saturating_sub(name_width)) / 2;
    buf.set_span(name_x, name_area.y, &span, name_width);
}

/// Render filename with matched characters highlighted in `match_style`.
fn render_filename_with_highlight(
    area: Rect,
    buf: &mut Buffer,
    filename: &str,
    query: &str,
    match_style: Style,
    normal_style: Style,
) {
    let mut spans: Vec<Span> = Vec::new();
    let filename_lower = filename.to_lowercase();
    let query_chars: Vec<char> = query.to_lowercase().chars().collect();
    let mut qi = 0;

    let filename_chars: Vec<char> = filename.chars().collect();
    let filename_lower_chars: Vec<char> = filename_lower.chars().collect();

    for (i, ch) in filename_chars.iter().enumerate() {
        if qi < query_chars.len() && filename_lower_chars[i] == query_chars[qi] {
            spans.push(Span::styled(ch.to_string(), match_style));
            qi += 1;
        } else {
            spans.push(Span::styled(ch.to_string(), normal_style));
        }
    }

    let total_width: usize = spans.iter().map(|s| s.width()).sum();
    let start_x = area.x + area.width.saturating_sub(total_width as u16) / 2;
    let mut x = start_x;
    for span in &spans {
        let w = span.width() as u16;
        buf.set_span(x, area.y, span, w);
        x += w;
    }
}
