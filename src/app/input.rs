use super::*;

impl App {
    /// Handle a key event. Returns true if the app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        if self.delete.is_some() {
            return self.handle_delete_key(code);
        }
        if self.rename.is_some() {
            return self.handle_rename_key(code, modifiers);
        }

        match self.state {
            AppState::Browser => {
                // In search mode, delegate to search handler
                if self.search.is_some() {
                    return self.handle_search_key(code, modifiers);
                }

                match code {
                    KeyCode::Char('q') => return true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                    KeyCode::Char('L') | KeyCode::Char('l') => {
                        self.lang.toggle();
                    }
                    KeyCode::Char('s') => self.cycle_sort_mode(),
                    KeyCode::Char('f') => self.toggle_favorite_current(),
                    KeyCode::Char('F') => self.toggle_favorites_view(),
                    KeyCode::Char('/') | KeyCode::Char('\\') => {
                        let trigger = match code {
                            KeyCode::Char(c) => c,
                            _ => '/',
                        };
                        self.search = Some(SearchState::new(self.selected, trigger));
                        return false;
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        self.browser_focus = match self.browser_focus {
                            BrowserFocus::Gallery => BrowserFocus::Context,
                            BrowserFocus::Context => BrowserFocus::Gallery,
                        };
                    }
                    _ => match self.browser_focus {
                        BrowserFocus::Gallery => match code {
                            KeyCode::Left => self.navigate_left(),
                            KeyCode::Right => self.navigate_right(),
                            KeyCode::Up => self.navigate_up(),
                            KeyCode::Down => self.navigate_down(),
                            KeyCode::PageDown | KeyCode::Char(' ') => {
                                self.navigate_page_down(self.visible_rows)
                            }
                            KeyCode::PageUp => self.navigate_page_up(self.visible_rows),
                            KeyCode::Home => self.navigate_home(),
                            KeyCode::End => self.navigate_end(),
                            KeyCode::Enter => self.enter_fullscreen(),
                            KeyCode::Char('r') => self.start_rename_current(),
                            KeyCode::Char('d') => self.start_delete_current(),
                            _ => {}
                        },
                        BrowserFocus::Context => match code {
                            KeyCode::Left => self.enter_parent_directory(),
                            KeyCode::Right | KeyCode::Enter => {
                                self.enter_selected_context_directory()
                            }
                            KeyCode::Up => self.context_up(),
                            KeyCode::Down => self.context_down(),
                            KeyCode::Home => self.context_home(),
                            KeyCode::End => self.context_end(),
                            _ => {}
                        },
                    },
                }
            }
            AppState::Fullscreen => match code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => self.exit_fullscreen(),
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,
                KeyCode::Char('L') => {
                    self.lang.toggle();
                }
                KeyCode::Char('f') => self.toggle_favorite_current(),
                KeyCode::Char('F') => self.toggle_favorites_view(),
                KeyCode::Char('+') | KeyCode::Char('=') => self.zoom_in(),
                KeyCode::Char('-') => self.zoom_out(),
                KeyCode::Char('0') => self.zoom_reset(),
                KeyCode::Char('r') => self.start_rename_current(),
                KeyCode::Char('d') => self.start_delete_current(),
                KeyCode::Char('h') => self.pan_left(),
                KeyCode::Char('l') => self.pan_right(),
                KeyCode::Char('k') => self.pan_up(),
                KeyCode::Char('j') => self.pan_down(),
                KeyCode::Left => self.fullscreen_prev(),
                KeyCode::Right => self.fullscreen_next(),
                _ => {}
            },
        }
        false
    }

    pub(super) fn handle_search_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> bool {
        // Enter in search mode: open fullscreen for current match
        if code == KeyCode::Enter {
            self.search = None;
            self.enter_fullscreen();
            return false;
        }

        let search = self.search.as_mut().unwrap();
        match search.handle_key(code, _modifiers, &self.images) {
            SearchAction::JumpTo(idx) => {
                self.selected = idx;
                self.clamp_scroll(self.visible_rows.max(1));
                false
            }
            SearchAction::Cancel => {
                self.selected = self.search.as_ref().unwrap().saved_selected;
                self.clamp_scroll(self.visible_rows.max(1));
                self.search = None;
                false
            }
            SearchAction::Continue => false,
        }
    }
}
