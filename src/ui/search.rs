use crossterm::event::{KeyCode, KeyModifiers};

use crate::lang::Lang;
use crate::scanner::ImageEntry;

pub struct SearchState {
    pub query: String,
    pub matches: Vec<usize>,
    pub match_idx: usize,
    pub saved_selected: usize,
    pub trigger_char: char,
}

pub enum SearchAction {
    Continue,
    JumpTo(usize),
    Cancel,
}

impl SearchState {
    pub fn new(current_selected: usize, trigger: char) -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            match_idx: 0,
            saved_selected: current_selected,
            trigger_char: trigger,
        }
    }

    pub fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        images: &[ImageEntry],
    ) -> SearchAction {
        match code {
            KeyCode::Esc => SearchAction::Cancel,
            KeyCode::Backspace => {
                self.query.pop();
                self.update_matches(images);
                self.jump_to_best()
            }
            KeyCode::Tab => {
                if !self.matches.is_empty() {
                    self.match_idx = (self.match_idx + 1) % self.matches.len();
                    SearchAction::JumpTo(self.matches[self.match_idx])
                } else {
                    SearchAction::Continue
                }
            }
            KeyCode::BackTab => {
                if !self.matches.is_empty() {
                    let n = self.matches.len();
                    self.match_idx = (self.match_idx + n - 1) % n;
                    SearchAction::JumpTo(self.matches[self.match_idx])
                } else {
                    SearchAction::Continue
                }
            }
            KeyCode::Char(c) => {
                self.query.push(c);
                self.update_matches(images);
                self.jump_to_best()
            }
            _ => SearchAction::Continue,
        }
    }

    fn jump_to_best(&mut self) -> SearchAction {
        if self.matches.is_empty() {
            SearchAction::Continue
        } else {
            self.match_idx = 0;
            SearchAction::JumpTo(self.matches[0])
        }
    }

    fn update_matches(&mut self, images: &[ImageEntry]) {
        let query_lower = self.query.to_lowercase();
        let mut scored: Vec<(usize, i32)> = images
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let score = fuzzy_score(&entry.filename, &query_lower)?;
                Some((idx, score))
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        self.matches = scored.into_iter().map(|(idx, _)| idx).collect();
    }
}

/// Fuzzy match: characters of `query` appear in `text` in order (case-insensitive).
/// Returns Some(score) if matched, None otherwise.
/// Higher score = better match. Consecutive chars + early position = bonus.
fn fuzzy_score(text: &str, query: &str) -> Option<i32> {
    let text_lower = text.to_lowercase();
    let text_chars: Vec<char> = text_lower.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return None;
    }

    let mut score: i32 = 0;
    let mut qi = 0;
    let mut prev_pos: Option<usize> = None;

    for (ti, tc) in text_chars.iter().enumerate() {
        if qi >= query_chars.len() {
            break;
        }
        if *tc == query_chars[qi] {
            score += 100 - (ti as i32).min(99);

            if let Some(prev) = prev_pos {
                if ti == prev + 1 {
                    score += 50; // consecutive bonus
                } else {
                    let gap = (ti - prev) as i32;
                    score -= gap.min(10); // gap penalty
                }
            }

            prev_pos = Some(ti);
            qi += 1;
        }
    }

    if qi == query_chars.len() {
        Some(score)
    } else {
        None
    }
}

// ---- SearchBar widget ----

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Paragraph, Widget},
};

pub struct SearchBar<'a> {
    pub state: &'a SearchState,
    pub lang: Lang,
}

impl<'a> Widget for SearchBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        let prompt = self.state.trigger_char;
        let query = &self.state.query;
        let total_matches = self.state.matches.len();
        let current = if total_matches > 0 {
            self.state.match_idx + 1
        } else {
            0
        };

        let cursor = "█";
        let display_query = format!("{}{}{}", prompt, query, cursor);

        let hint = if total_matches > 0 {
            self.lang.search_hint_matches(current, total_matches)
        } else if query.is_empty() {
            self.lang.search_hint_empty().to_string()
        } else {
            self.lang.search_hint_none().to_string()
        };

        let query_style = if total_matches == 0 && !query.is_empty() {
            Style::default().fg(Color::Red).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };

        let hint_style = Style::default().fg(Color::Gray).bg(Color::DarkGray);

        let spans = vec![
            Span::styled(display_query, query_style),
            Span::styled(hint, hint_style),
        ];

        Paragraph::new(ratatui::text::Line::from(spans))
            .alignment(Alignment::Left)
            .render(area, buf);
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_images(names: &[&str]) -> Vec<ImageEntry> {
        names
            .iter()
            .map(|name| ImageEntry {
                path: PathBuf::from(name),
                filename: name.to_string(),
                file_size: 0,
            })
            .collect()
    }

    fn cell_text(buf: &Buffer, area: Rect) -> String {
        let mut s = String::new();
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell((x, area.y)) {
                s.push_str(&cell.symbol());
            }
        }
        s.trim_end().to_string()
    }

    #[test]
    fn test_searchbar_with_no_query() {
        let state = SearchState::new(0, '/');
        let bar = SearchBar {
            state: &state,
            lang: Lang::Zh,
        };
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 1,
        };
        let mut buf = Buffer::empty(area);
        bar.render(area, &mut buf);
        let content = cell_text(&buf, area);
        assert!(
            content.starts_with('/'),
            "expected leading '/', got: {content:?}"
        );
        assert!(
            content.contains("Esc"),
            "expected Esc hint, got: {content:?}"
        );
    }

    #[test]
    fn test_search_new_saves_selected() {
        let s = SearchState::new(5, '/');
        assert_eq!(s.saved_selected, 5);
        assert_eq!(s.trigger_char, '/');
        assert!(s.query.is_empty());
        assert!(s.matches.is_empty());
        assert_eq!(s.match_idx, 0);
    }

    #[test]
    fn test_fuzzy_match_substring() {
        let images = make_images(&["abc.png", "xyz.png", "bcd.png"]);
        let mut s = SearchState::new(0, '/');
        let action = s.handle_key(KeyCode::Char('b'), KeyModifiers::NONE, &images);
        assert!(matches!(action, SearchAction::JumpTo(_)));
        assert!(!s.matches.is_empty());
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        let images = make_images(&["Photo.PNG", "image.jpg"]);
        let mut s = SearchState::new(0, '/');
        let action = s.handle_key(KeyCode::Char('p'), KeyModifiers::NONE, &images);
        assert!(matches!(action, SearchAction::JumpTo(_)));
        assert!(!s.matches.is_empty());
    }

    #[test]
    fn test_fuzzy_match_no_results() {
        let images = make_images(&["abc.png", "def.jpg"]);
        let mut s = SearchState::new(0, '/');
        let action = s.handle_key(KeyCode::Char('z'), KeyModifiers::NONE, &images);
        assert!(matches!(action, SearchAction::Continue));
        assert!(s.matches.is_empty());
        assert_eq!(s.query, "z");
    }

    #[test]
    fn test_backspace_removes_char() {
        let images = make_images(&["abc.png"]);
        let mut s = SearchState::new(0, '/');
        s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, &images);
        s.handle_key(KeyCode::Char('b'), KeyModifiers::NONE, &images);
        assert_eq!(s.query, "ab");
        let action = s.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &images);
        assert_eq!(s.query, "a");
        assert!(matches!(action, SearchAction::JumpTo(_)));
    }

    #[test]
    fn test_tab_cycles_forward() {
        let images = make_images(&["abc.png", "abc.jpg", "abd.png"]);
        let mut s = SearchState::new(0, '/');
        s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, &images);
        let first = s.match_idx;
        let action = s.handle_key(KeyCode::Tab, KeyModifiers::NONE, &images);
        assert!(matches!(action, SearchAction::JumpTo(_)));
        assert_eq!(s.match_idx, (first + 1) % s.matches.len());
    }

    #[test]
    fn test_backtab_cycles_backward() {
        let images = make_images(&["abc.png", "abc.jpg", "abd.png"]);
        let mut s = SearchState::new(0, '/');
        s.handle_key(KeyCode::Char('a'), KeyModifiers::NONE, &images);
        let n = s.matches.len();
        let first = s.match_idx;
        let action = s.handle_key(KeyCode::BackTab, KeyModifiers::NONE, &images);
        assert!(matches!(action, SearchAction::JumpTo(_)));
        assert_eq!(s.match_idx, (first + n - 1) % n);
    }

    #[test]
    fn test_esc_cancels() {
        let mut s = SearchState::new(5, '/');
        let action = s.handle_key(KeyCode::Esc, KeyModifiers::NONE, &[]);
        assert!(matches!(action, SearchAction::Cancel));
    }
}
