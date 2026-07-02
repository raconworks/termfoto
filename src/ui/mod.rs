pub mod browser;
pub mod layout;
pub mod preview;
pub mod search;

use crate::app::{App, AppState, DirectoryContextEntry, LOGO_HEIGHT};
use crate::lang::Lang;
use crate::scanner::ImageEntry;
use crate::ui::browser::BrowserView;
use crate::ui::preview::PreviewView;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMode {
    Gallery,
    Folder,
    Favorites,
    View,
    Status,
    Search,
    Rename,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptLineKind {
    Normal,
    Error,
    Warning,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePrompt {
    pub lang: Lang,
    pub original_name: String,
    pub input: String,
    pub message: Option<String>,
    pub pending_overwrite: bool,
    pub target_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletePrompt {
    pub lang: Lang,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultPrompt {
    pub lang: Lang,
    pub kind: DefaultPromptKind,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultPromptKind {
    Gallery {
        name: String,
        selected: usize,
        total: usize,
        sort_label: String,
    },
    Folder {
        name: String,
        selected: usize,
        total: usize,
        sort_label: String,
    },
    Favorites {
        name: String,
        selected: usize,
        total: usize,
    },
    View {
        name: String,
        selected: usize,
        total: usize,
        loading: bool,
        zoom_percent: u16,
        favorites_view: bool,
    },
}

pub fn prompt_base_style() -> Style {
    Style::default().bg(Color::DarkGray)
}

pub fn prompt_label_style() -> Style {
    prompt_base_style()
        .fg(Color::Black)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_input_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_status_style() -> Style {
    prompt_base_style().fg(Color::White)
}

pub fn prompt_hint_style() -> Style {
    prompt_base_style().fg(Color::Gray)
}

pub fn prompt_key_style() -> Style {
    prompt_base_style()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_value_style() -> Style {
    prompt_base_style()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_chip_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Gray)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_error_style() -> Style {
    prompt_base_style()
        .fg(Color::Red)
        .add_modifier(Modifier::BOLD)
}

pub fn prompt_warning_style() -> Style {
    prompt_base_style()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

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

pub fn render_panel(area: Rect, title: &str, focused: bool, buf: &mut Buffer) -> Rect {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    block.render(area, buf);
    inner
}

pub fn render_directory_context(
    area: Rect,
    entries: &[DirectoryContextEntry],
    empty_text: &str,
    selected: Option<usize>,
    scroll: usize,
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
        .enumerate()
        .skip(scroll)
        .take(area.height as usize)
        .map(|(idx, entry)| {
            let is_selected = selected == Some(idx);
            let marker = if is_selected || entry.is_current {
                "> "
            } else {
                "  "
            };
            let indent = "    ".repeat(entry.depth);
            let text =
                truncate_to_width(&format!("{}{}{}/", indent, marker, entry.name), area.width);
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if entry.is_current {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
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

pub fn render_prompt_rich_lines(area: Rect, lines: &[Line<'_>], buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    render_prompt_base(area, buf);
    let prompt_lines: Vec<Line> = lines
        .iter()
        .take(area.height as usize)
        .map(|line| truncate_rich_line(line, area.width))
        .collect();
    Paragraph::new(prompt_lines).render(area, buf);
}

pub fn render_prompt_base(area: Rect, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    buf.set_style(area, prompt_base_style());
    render_logo(area, buf);
}

pub fn render_rename_prompt(area: Rect, prompt: &RenamePrompt, buf: &mut Buffer) {
    render_prompt_rich_lines(area, &rename_prompt_lines(prompt), buf);
}

pub fn render_delete_prompt(area: Rect, prompt: &DeletePrompt, buf: &mut Buffer) {
    render_prompt_rich_lines(area, &delete_prompt_lines(prompt), buf);
}

pub fn render_default_prompt(area: Rect, prompt: &DefaultPrompt, buf: &mut Buffer) {
    let lines = default_prompt_lines(prompt, area.width);
    render_prompt_rich_lines(area, &lines, buf);
}

pub fn prompt_mode_label(mode: PromptMode) -> Span<'static> {
    let label = match mode {
        PromptMode::Gallery => " GALLERY ",
        PromptMode::Folder => " FOLDER ",
        PromptMode::Favorites => " FAVORITES ",
        PromptMode::View => " VIEW ",
        PromptMode::Status => " STATUS ",
        PromptMode::Search => " SEARCH ",
        PromptMode::Rename => " RENAME ",
        PromptMode::Delete => " DELETE ",
    };
    Span::styled(label, prompt_label_style())
}

pub fn prompt_text_span(text: impl Into<String>, kind: PromptLineKind) -> Span<'static> {
    let style = match kind {
        PromptLineKind::Normal => prompt_status_style(),
        PromptLineKind::Error => prompt_error_style(),
        PromptLineKind::Warning => prompt_warning_style(),
        PromptLineKind::Hint => prompt_hint_style(),
    };
    Span::styled(text.into(), style)
}

pub fn prompt_input_span(input: impl Into<String>) -> Span<'static> {
    Span::styled(input.into(), prompt_input_style())
}

pub fn prompt_key_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), prompt_key_style())
}

pub fn prompt_value_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), prompt_value_style())
}

pub fn prompt_muted_span(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), prompt_hint_style())
}

pub fn prompt_state_chip(text: impl Into<String>) -> Span<'static> {
    Span::styled(format!(" {} ", text.into()), prompt_chip_style())
}

fn default_prompt_lines(prompt: &DefaultPrompt, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(3);
    if let Some(message) = &prompt.status_message {
        lines.push(Line::from(vec![
            prompt_mode_label(PromptMode::Status),
            prompt_text_span(format!("  {}", message), PromptLineKind::Warning),
        ]));
    } else {
        lines.push(default_prompt_header_line(
            prompt.lang,
            &prompt.kind,
            width as usize,
        ));
    }

    let (navigation, actions) = default_prompt_shortcut_lines(prompt.lang, &prompt.kind);
    lines.push(navigation);
    lines.push(actions);
    lines
}

fn default_prompt_header_line(lang: Lang, kind: &DefaultPromptKind, width: usize) -> Line<'static> {
    let (mode, name, selected, total, chips) = match kind {
        DefaultPromptKind::Gallery {
            name,
            selected,
            total,
            sort_label,
        } => (
            PromptMode::Gallery,
            name.as_str(),
            *selected,
            *total,
            vec![sort_chip(lang, sort_label)],
        ),
        DefaultPromptKind::Folder {
            name,
            selected,
            total,
            sort_label,
        } => (
            PromptMode::Folder,
            name.as_str(),
            *selected,
            *total,
            vec![sort_chip(lang, sort_label)],
        ),
        DefaultPromptKind::Favorites {
            name,
            selected,
            total,
        } => (
            PromptMode::Favorites,
            name.as_str(),
            *selected,
            *total,
            Vec::new(),
        ),
        DefaultPromptKind::View {
            name,
            selected,
            total,
            loading,
            zoom_percent,
            ..
        } => {
            let state = if *loading {
                loading_chip(lang)
            } else {
                zoom_chip(lang, *zoom_percent)
            };
            (
                PromptMode::View,
                name.as_str(),
                *selected,
                *total,
                vec![state],
            )
        }
    };

    let label = prompt_mode_label(mode);
    let gap = prompt_muted_span("  ");
    let suffix_with_chips = default_header_suffix(selected, total, &chips);
    let suffix_without_chips = default_header_suffix(selected, total, &[]);
    let mut fixed_with_chips = vec![label.clone(), gap.clone()];
    fixed_with_chips.extend(suffix_with_chips.clone());
    let suffix = if spans_width(&fixed_with_chips) <= width {
        suffix_with_chips
    } else {
        suffix_without_chips
    };
    let mut fixed_spans = vec![label.clone(), gap.clone()];
    fixed_spans.extend(suffix.clone());
    let fixed_width = spans_width(&fixed_spans);
    let name_width = width.saturating_sub(fixed_width);
    let mut spans = vec![
        label,
        gap,
        prompt_value_span(truncate_text_to_width(name, name_width)),
    ];
    spans.extend(suffix);
    Line::from(spans)
}

fn default_header_suffix(selected: usize, total: usize, chips: &[String]) -> Vec<Span<'static>> {
    let mut suffix = vec![
        prompt_muted_span("  "),
        prompt_text_span(format!("[{}/{}]", selected, total), PromptLineKind::Normal),
    ];
    for chip in chips {
        suffix.push(prompt_muted_span("  "));
        suffix.push(prompt_state_chip(chip));
    }
    suffix
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    Line::from(spans.to_vec()).width()
}

fn sort_chip(lang: Lang, sort_label: &str) -> String {
    match lang {
        Lang::Zh => format!("排列 {}", sort_label),
        Lang::En => format!("Sort {}", sort_label),
    }
}

fn loading_chip(lang: Lang) -> String {
    match lang {
        Lang::Zh => "加载中".to_string(),
        Lang::En => "Loading".to_string(),
    }
}

fn zoom_chip(lang: Lang, zoom_percent: u16) -> String {
    match lang {
        Lang::Zh => format!("缩放 {}%", zoom_percent),
        Lang::En => format!("Zoom {}%", zoom_percent),
    }
}

fn default_prompt_shortcut_lines(
    lang: Lang,
    kind: &DefaultPromptKind,
) -> (Line<'static>, Line<'static>) {
    match kind {
        DefaultPromptKind::Gallery { .. } => match lang {
            Lang::Zh => (
                shortcut_line(
                    " 导航     ",
                    &[
                        ("←→↑↓", "导航"),
                        ("PgUp/PgDown/Space", "翻页"),
                        ("Home/End", "首尾"),
                    ],
                ),
                shortcut_line(
                    " 操作     ",
                    &[
                        ("Enter", "全屏"),
                        ("Tab", "切换面板"),
                        ("/", "搜索"),
                        ("r", "重命名"),
                        ("d", "删除"),
                        ("f", "收藏"),
                        ("F", "收藏视图"),
                        ("s", "排列"),
                        ("L", "切换语言"),
                        ("q", "退出"),
                    ],
                ),
            ),
            Lang::En => (
                shortcut_line(
                    " Move     ",
                    &[
                        ("←→↑↓", "Nav"),
                        ("PgUp/PgDown/Space", "Page"),
                        ("Home/End", "First/Last"),
                    ],
                ),
                shortcut_line(
                    " Action   ",
                    &[
                        ("Enter", "View"),
                        ("Tab", "Focus"),
                        ("/", "Search"),
                        ("s", "Sort"),
                        ("r", "Rename"),
                        ("d", "Delete"),
                        ("f", "Fav"),
                        ("F", "Favs"),
                        ("L", "Lang"),
                        ("q", "Quit"),
                    ],
                ),
            ),
        },
        DefaultPromptKind::Folder { .. } => match lang {
            Lang::Zh => (
                shortcut_line(
                    " 导航     ",
                    &[("↑↓", "选择"), ("Home/End", "首尾"), ("←", "上一级")],
                ),
                shortcut_line(
                    " 操作     ",
                    &[
                        ("→/Enter", "进入文件夹"),
                        ("Tab", "切换面板"),
                        ("f", "收藏"),
                        ("F", "收藏视图"),
                        ("s", "排列"),
                        ("L", "切换语言"),
                        ("q", "退出"),
                    ],
                ),
            ),
            Lang::En => (
                shortcut_line(
                    " Move     ",
                    &[
                        ("↑↓", "Select"),
                        ("Home/End", "First/Last"),
                        ("←", "Parent"),
                    ],
                ),
                shortcut_line(
                    " Action   ",
                    &[
                        ("→/Enter", "Open Folder"),
                        ("Tab", "Focus"),
                        ("f", "Favorite"),
                        ("F", "Favorites"),
                        ("s", "Sort"),
                        ("L", "Language"),
                        ("q", "Quit"),
                    ],
                ),
            ),
        },
        DefaultPromptKind::Favorites { .. } => match lang {
            Lang::Zh => (
                shortcut_line(
                    " 导航     ",
                    &[
                        ("←→↑↓", "导航"),
                        ("PgUp/PgDown/Space", "翻页"),
                        ("Home/End", "首尾"),
                    ],
                ),
                shortcut_line(
                    " 操作     ",
                    &[
                        ("Enter", "全屏"),
                        ("/", "搜索"),
                        ("d", "删除"),
                        ("f", "取消收藏"),
                        ("F", "返回图库"),
                        ("L", "切换语言"),
                        ("q", "退出"),
                    ],
                ),
            ),
            Lang::En => (
                shortcut_line(
                    " Move     ",
                    &[
                        ("←→↑↓", "Nav"),
                        ("PgUp/PgDown/Space", "Page"),
                        ("Home/End", "First/Last"),
                    ],
                ),
                shortcut_line(
                    " Action   ",
                    &[
                        ("Enter", "View"),
                        ("/", "Search"),
                        ("d", "Delete"),
                        ("f", "Unfav"),
                        ("F", "Gallery"),
                        ("L", "Lang"),
                        ("q", "Quit"),
                    ],
                ),
            ),
        },
        DefaultPromptKind::View { favorites_view, .. } => {
            let switch = match (lang, favorites_view) {
                (Lang::Zh, true) => ("F", "返回图库"),
                (Lang::Zh, false) => ("F", "收藏视图"),
                (Lang::En, true) => ("F", "Gallery"),
                (Lang::En, false) => ("F", "Favorites"),
            };
            match lang {
                Lang::Zh => (
                    shortcut_line(
                        " 导航     ",
                        &[
                            ("←/→", "切换图片"),
                            ("+/-", "缩放"),
                            ("0", "重置"),
                            ("hjkl", "平移"),
                        ],
                    ),
                    shortcut_line(
                        " 操作     ",
                        &[
                            ("r", "重命名"),
                            ("d", "删除"),
                            ("f", "收藏"),
                            switch,
                            ("Enter/Esc/q", "返回"),
                            ("L", "语言"),
                        ],
                    ),
                ),
                Lang::En => (
                    shortcut_line(
                        " Move     ",
                        &[
                            ("←/→", "Prev/Next"),
                            ("+/-", "Zoom"),
                            ("0", "Reset"),
                            ("hjkl", "Pan"),
                        ],
                    ),
                    shortcut_line(
                        " Action   ",
                        &[
                            ("r", "Rename"),
                            ("d", "Delete"),
                            ("f", "Fav"),
                            switch,
                            ("Enter/Esc/q", "Back"),
                            ("L", "Lang"),
                        ],
                    ),
                ),
            }
        }
    }
}

fn shortcut_line(prefix: &'static str, items: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = vec![prompt_muted_span(prefix)];
    for (idx, (key, label)) in items.iter().enumerate() {
        if idx > 0 {
            spans.push(prompt_muted_span("   "));
        }
        spans.push(prompt_key_span(*key));
        spans.push(prompt_muted_span(format!(" {}", label)));
    }
    Line::from(spans)
}

fn rename_prompt_lines(prompt: &RenamePrompt) -> Vec<Line<'static>> {
    let (
        status_prefix,
        action_prefix,
        edit_status,
        overwrite_status,
        save_action,
        overwrite_action,
    ) = match prompt.lang {
        Lang::Zh => (
            " 状态     ",
            " 操作     ",
            "输入新文件名（不含扩展名）",
            "目标已存在，覆盖？",
            "Enter 保存   Esc 取消",
            "y 确认   n/Esc 取消",
        ),
        Lang::En => (
            " Status   ",
            " Action   ",
            "Enter a new filename stem",
            "Target exists. Overwrite?",
            "Enter Save   Esc Cancel",
            "y Confirm   n/Esc Cancel",
        ),
    };

    let first_line = if prompt.pending_overwrite {
        Line::from(vec![
            prompt_mode_label(PromptMode::Rename),
            prompt_text_span(
                format!("  {} -> ", prompt.original_name),
                PromptLineKind::Normal,
            ),
            prompt_input_span(format!(
                "[{}]",
                prompt.target_name.as_deref().unwrap_or(&prompt.input)
            )),
        ])
    } else {
        Line::from(vec![
            prompt_mode_label(PromptMode::Rename),
            prompt_text_span(
                format!("  {} -> ", prompt.original_name),
                PromptLineKind::Normal,
            ),
            prompt_input_span(format!("[{}█]", prompt.input)),
        ])
    };

    let status_line = if prompt.pending_overwrite {
        Line::from(prompt_text_span(
            format!("{}{}", status_prefix, overwrite_status),
            PromptLineKind::Warning,
        ))
    } else if let Some(message) = &prompt.message {
        Line::from(prompt_text_span(
            format!("{}{}", status_prefix, message),
            PromptLineKind::Error,
        ))
    } else {
        Line::from(prompt_text_span(
            format!("{}{}", status_prefix, edit_status),
            PromptLineKind::Normal,
        ))
    };

    let action_line = if prompt.pending_overwrite {
        Line::from(prompt_text_span(
            format!("{}{}", action_prefix, overwrite_action),
            PromptLineKind::Hint,
        ))
    } else {
        Line::from(prompt_text_span(
            format!("{}{}", action_prefix, save_action),
            PromptLineKind::Hint,
        ))
    };

    vec![first_line, status_line, action_line]
}

fn delete_prompt_lines(prompt: &DeletePrompt) -> Vec<Line<'static>> {
    let (status, action) = match prompt.lang {
        Lang::Zh => (
            " 状态     永久删除此图片？",
            " 操作     y/Enter 确认   n/Esc 取消",
        ),
        Lang::En => (
            " Status   Permanently delete this image?",
            " Action   y/Enter Confirm   n/Esc Cancel",
        ),
    };

    vec![
        Line::from(vec![
            prompt_mode_label(PromptMode::Delete),
            prompt_text_span(format!("  {}", prompt.filename), PromptLineKind::Normal),
        ]),
        Line::from(prompt_text_span(status, PromptLineKind::Warning)),
        Line::from(prompt_text_span(action, PromptLineKind::Hint)),
    ]
}

fn truncate_rich_line(line: &Line<'_>, width: u16) -> Line<'static> {
    let max = width as usize;
    if max == 0 {
        return Line::default();
    }

    if line.width() <= max {
        return Line {
            style: line.style,
            alignment: line.alignment,
            spans: line
                .spans
                .iter()
                .map(|span| Span::styled(span.content.to_string(), span.style))
                .collect(),
        };
    }

    let mut remaining = max;
    let mut spans = Vec::new();
    for span in &line.spans {
        if remaining == 0 {
            break;
        }

        let span_width = span.width();
        if span_width <= remaining {
            spans.push(Span::styled(span.content.to_string(), span.style));
            remaining -= span_width;
            continue;
        }

        if remaining == 1 {
            spans.push(Span::styled("…", span.style));
            break;
        }

        let mut text = String::new();
        let mut used = 0;
        let content_limit = remaining - 1;
        for ch in span.content.chars() {
            let ch_text = ch.to_string();
            let ch_width = Span::raw(ch_text.as_str()).width();
            if used + ch_width > content_limit {
                break;
            }
            text.push(ch);
            used += ch_width;
        }
        text.push('…');
        spans.push(Span::styled(text, span.style));
        break;
    }

    Line {
        style: line.style,
        alignment: line.alignment,
        spans,
    }
}

fn truncate_text_to_width(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if Span::raw(text).width() <= max {
        return text.to_string();
    }
    if max == 1 {
        return "…".to_string();
    }

    let mut truncated = String::new();
    let mut used = 0;
    let content_limit = max - 1;
    for ch in text.chars() {
        let ch_text = ch.to_string();
        let ch_width = Span::raw(ch_text.as_str()).width();
        if used + ch_width > content_limit {
            break;
        }
        truncated.push(ch);
        used += ch_width;
    }
    truncated.push('…');
    truncated
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

    fn buffer_text(buf: &Buffer) -> String {
        buf.content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    fn row_text(buf: &Buffer, area: Rect, y: u16) -> String {
        (area.x..area.x + area.width)
            .map(|x| buf.cell((x, y)).unwrap().symbol())
            .collect()
    }

    #[test]
    fn compact_logo_uses_three_rows() {
        assert_eq!(LOGO_HEIGHT, 3);
        assert_eq!(LOGO_LINES.len(), LOGO_HEIGHT as usize);
        assert_eq!(LOGO_COLORS.len(), LOGO_HEIGHT as usize);
    }

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
            modified_at: None,
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

    #[test]
    fn rename_prompt_styles_input_inside_brackets() {
        let prompt = RenamePrompt {
            lang: Lang::En,
            original_name: "old.png".to_string(),
            input: "new-name".to_string(),
            message: None,
            pending_overwrite: false,
            target_name: None,
        };
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);

        render_rename_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("RENAME"));
        assert!(text.contains("[new-name█]"));
        assert!(text.contains("Enter Save"));

        let input_cell = (area.x..area.x + area.width)
            .find_map(|x| {
                let cell = buf.cell((x, area.y)).unwrap();
                (cell.symbol() == "█").then_some(cell)
            })
            .expect("input should be rendered");
        assert_eq!(input_cell.bg, Color::White);
    }

    #[test]
    fn delete_prompt_shows_mode_and_confirmation_keys() {
        let prompt = DeletePrompt {
            lang: Lang::En,
            filename: "old.png".to_string(),
        };
        let area = Rect::new(0, 0, 80, 3);
        let mut buf = Buffer::empty(area);

        render_delete_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("DELETE"));
        assert!(text.contains("old.png"));
        assert!(text.contains("y/Enter"));
        assert!(text.contains("n/Esc"));

        let warning_cell = buf.cell((area.x + 1, area.y + 1)).unwrap();
        assert_eq!(warning_cell.fg, Color::Yellow);
    }

    #[test]
    fn default_gallery_prompt_shows_mode_name_count_and_sort_chip() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::Gallery {
                name: "sample.png".to_string(),
                selected: 1,
                total: 10,
                sort_label: "Name".to_string(),
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("GALLERY"));
        assert!(text.contains("sample.png"));
        assert!(text.contains("[1/10]"));
        assert!(text.contains("Sort Name"));
        assert!(text.contains("Enter View"));

        let label_cell = buf.cell((1, 0)).unwrap();
        assert_eq!(label_cell.bg, Color::Cyan);

        let key_cell = (area.x..area.x + area.width)
            .find_map(|x| {
                let cell = buf.cell((x, area.y + 2)).unwrap();
                (cell.symbol() == "E").then_some(cell)
            })
            .expect("Enter key should be rendered");
        assert_eq!(key_cell.fg, Color::Yellow);
    }

    #[test]
    fn default_folder_prompt_shows_folder_actions() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::Folder {
                name: "photos".to_string(),
                selected: 2,
                total: 4,
                sort_label: "Modified".to_string(),
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("FOLDER"));
        assert!(text.contains("photos"));
        assert!(text.contains("[2/4]"));
        assert!(text.contains("Sort Modified"));
        assert!(text.contains("Open Folder"));
    }

    #[test]
    fn default_favorites_prompt_shows_unfavorite_and_gallery_actions() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::Favorites {
                name: "liked.png".to_string(),
                selected: 1,
                total: 3,
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("FAVORITES"));
        assert!(text.contains("liked.png"));
        assert!(text.contains("[1/3]"));
        assert!(text.contains("Unfav"));
        assert!(text.contains("F Gallery"));
    }

    #[test]
    fn default_view_prompt_shows_zoom_chip_and_view_actions() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::View {
                name: "full.png".to_string(),
                selected: 4,
                total: 9,
                loading: false,
                zoom_percent: 125,
                favorites_view: false,
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("VIEW"));
        assert!(text.contains("full.png"));
        assert!(text.contains("[4/9]"));
        assert!(text.contains("Zoom 125%"));
        assert!(text.contains("Enter/Esc/q Back"));
    }

    #[test]
    fn default_view_prompt_can_show_loading_chip() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::View {
                name: "full.png".to_string(),
                selected: 4,
                total: 9,
                loading: true,
                zoom_percent: 100,
                favorites_view: true,
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("Loading"));
        assert!(text.contains("F Gallery"));
    }

    #[test]
    fn default_status_prompt_keeps_context_shortcuts() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::Gallery {
                name: "sample.png".to_string(),
                selected: 1,
                total: 10,
                sort_label: "Name".to_string(),
            },
            status_message: Some("Favorited".to_string()),
        };
        let area = Rect::new(0, 0, 100, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let text = buffer_text(&buf);
        assert!(text.contains("STATUS"));
        assert!(text.contains("Favorited"));
        assert!(text.contains("PgUp/PgDown/Space Page"));
        assert!(text.contains("Enter View"));
    }

    #[test]
    fn default_prompt_truncates_long_name_before_count_and_chip() {
        let prompt = DefaultPrompt {
            lang: Lang::En,
            kind: DefaultPromptKind::Gallery {
                name: "an-extremely-long-filename-that-would-hide-metadata.png".to_string(),
                selected: 12,
                total: 345,
                sort_label: "Modified".to_string(),
            },
            status_message: None,
        };
        let area = Rect::new(0, 0, 48, 3);
        let mut buf = Buffer::empty(area);

        render_default_prompt(area, &prompt, &mut buf);

        let first_row = row_text(&buf, area, 0);
        assert!(first_row.contains("GALLERY"));
        assert!(first_row.contains("…"));
        assert!(first_row.contains("[12/345]"));
        assert!(first_row.contains("Sort Modified"));
        assert_eq!(first_row.chars().count(), area.width as usize);
    }

    #[test]
    fn rich_prompt_truncation_preserves_width_limit() {
        let line = Line::from(vec![
            Span::styled("0123456789", prompt_status_style()),
            Span::styled("abcdef", prompt_hint_style()),
        ]);

        let truncated = truncate_rich_line(&line, 8);

        assert!(truncated.width() <= 8);
        let rendered = String::from(truncated);
        assert_eq!(rendered, "0123456…");
    }
}
