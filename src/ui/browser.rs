use crate::app::{App, BrowserFocus};
use crate::ui::layout::three_panel_areas;
use crate::ui::search::SearchBar;
use crate::ui::{
    render_directory_context, render_info_panel, render_panel, render_prompt_base,
    render_prompt_lines,
};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

mod cell;
mod grid;
mod thumbnails;

#[cfg(test)]
mod tests;

pub use thumbnails::populate_protocol_cache;

pub struct BrowserView<'a> {
    pub app: &'a mut App,
    pub cell_w: u16,
    pub cell_h: u16,
}

impl<'a> Widget for BrowserView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let cell_w = self.cell_w.max(1);
        let cell_h = self.cell_h.max(1);

        let areas = three_panel_areas(area);
        let context_inner = render_panel(
            areas.context,
            self.app.lang.title_context(),
            self.app.browser_focus == BrowserFocus::Context,
            buf,
        );
        let favorite_row_len = self.app.favorite_row_len();
        let has_favorites_area = favorite_row_len > 0;
        let favorites_height = if has_favorites_area {
            cell_h.saturating_add(2).min(areas.gallery.height)
        } else {
            0
        };
        let favorites_area = Rect {
            height: favorites_height,
            ..areas.gallery
        };
        let gallery_area = Rect {
            y: areas.gallery.y + favorites_height,
            height: areas.gallery.height.saturating_sub(favorites_height),
            ..areas.gallery
        };
        let selected_favorite = has_favorites_area && self.app.selected < favorite_row_len;
        let favorites_inner = has_favorites_area.then(|| {
            render_panel(
                favorites_area,
                self.app.lang.title_favorites(),
                self.app.browser_focus == BrowserFocus::Gallery && selected_favorite,
                buf,
            )
        });
        let gallery_inner = render_panel(
            gallery_area,
            self.app.lang.title_gallery(),
            self.app.browser_focus == BrowserFocus::Gallery
                && (!has_favorites_area || !selected_favorite),
            buf,
        );
        let info_inner = render_panel(areas.info, self.app.lang.title_info(), false, buf);

        let context_entries = self.app.directory_context_for_browser();
        self.app
            .clamp_context_selection(context_entries.len(), context_inner.height as usize);
        render_directory_context(
            context_inner,
            &context_entries,
            self.app.lang.empty_folder_context(),
            Some(self.app.context_selected),
            self.app.context_scroll,
            buf,
        );
        render_info_panel(
            info_inner,
            self.app.images.get(self.app.selected),
            None,
            self.app,
            buf,
        );

        let gallery_visible_rows = (gallery_inner.height / cell_h).max(1) as usize;
        let visible_rows = gallery_visible_rows + usize::from(has_favorites_area);
        self.app.visible_rows = visible_rows;

        self.app.clamp_scroll(visible_rows);

        let search_matches: Option<&[usize]> =
            self.app.search.as_ref().map(|s| s.matches.as_slice());

        if let Some(favorites_inner) = favorites_inner {
            let favorite_grid =
                grid::gallery_grid(favorites_inner, self.app.grid_cols, cell_w, cell_h, 1);
            for slot in 0..favorite_row_len {
                let col = slot as u16;
                grid::render_gallery_slot(
                    self.app,
                    slot,
                    col,
                    0,
                    &favorite_grid,
                    search_matches,
                    buf,
                );
            }
        }

        let normal_visible_rows = self.app.normal_visible_rows(visible_rows);
        let grid = grid::gallery_grid(
            gallery_inner,
            self.app.grid_cols,
            cell_w,
            cell_h,
            gallery_visible_rows,
        );

        let start = favorite_row_len + self.app.scroll_row * self.app.grid_cols;
        let end = (start + normal_visible_rows * self.app.grid_cols).min(self.app.images.len());

        for slot in start..end {
            let vis_idx = slot - start;
            let col = (vis_idx % self.app.grid_cols) as u16;
            let row = (vis_idx / self.app.grid_cols) as u16;
            grid::render_gallery_slot(self.app, slot, col, row, &grid, search_matches, buf);
        }

        if let Some(lines) = self.app.delete_prompt_lines() {
            render_prompt_lines(areas.prompt, &lines, buf);
        } else if let Some(lines) = self.app.rename_prompt_lines() {
            render_prompt_lines(areas.prompt, &lines, buf);
        } else if let Some(ref search) = self.app.search {
            render_prompt_base(areas.prompt, buf);
            SearchBar {
                state: search,
                lang: self.app.lang,
            }
            .render(areas.prompt, buf);
        } else {
            let selected_name = self
                .app
                .images
                .get(self.app.selected)
                .map(|e| e.filename.as_str())
                .unwrap_or("");
            let mut lines = if self.app.is_favorites_view() {
                self.app.lang.favorites_prompt_lines(
                    selected_name,
                    self.app
                        .selected
                        .saturating_add(1)
                        .min(self.app.images.len()),
                    self.app.images.len(),
                )
            } else {
                match self.app.browser_focus {
                    BrowserFocus::Gallery => self.app.lang.browser_prompt_lines(
                        selected_name,
                        self.app
                            .selected
                            .saturating_add(1)
                            .min(self.app.images.len()),
                        self.app.images.len(),
                        self.app.sort_label(),
                    ),
                    BrowserFocus::Context => {
                        let context_name = context_entries
                            .get(self.app.context_selected)
                            .map(|entry| entry.name.as_str())
                            .unwrap_or("");
                        self.app.lang.context_prompt_lines(
                            context_name,
                            self.app
                                .context_selected
                                .saturating_add(1)
                                .min(context_entries.len()),
                            context_entries.len(),
                            self.app.sort_label(),
                        )
                    }
                }
            };
            if let Some(message) = self.app.browser_status_message() {
                lines[0] = self.app.lang.status_prompt_line(&message);
            }
            render_prompt_lines(areas.prompt, &lines, buf);
        }
    }
}
