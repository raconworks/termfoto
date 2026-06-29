use ratatui::layout::{Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreePanelAreas {
    pub context: Rect,
    pub gallery: Rect,
    pub info: Rect,
    pub prompt: Rect,
}

pub const PROMPT_HEIGHT: u16 = 3;

pub fn three_panel_areas(area: Rect) -> ThreePanelAreas {
    let prompt_h = area.height.min(PROMPT_HEIGHT);
    let main_h = area.height.saturating_sub(prompt_h);

    let context_w = area.width / 5;
    let gallery_w = area.width.saturating_mul(3) / 5;
    let info_w = area.width.saturating_sub(context_w + gallery_w);

    let context = Rect {
        height: main_h,
        width: context_w,
        ..area
    };
    let gallery = Rect {
        x: area.x + context_w,
        height: main_h,
        width: gallery_w,
        ..area
    };
    let info = Rect {
        x: area.x + context_w + gallery_w,
        height: main_h,
        width: info_w,
        ..area
    };
    let prompt = Rect {
        y: area.y + main_h,
        height: prompt_h,
        ..area
    };

    ThreePanelAreas {
        context,
        gallery,
        info,
        prompt,
    }
}

pub fn gallery_inner_size(term_size: Size) -> Size {
    let areas = three_panel_areas(Rect {
        x: 0,
        y: 0,
        width: term_size.width,
        height: term_size.height,
    });
    Size {
        width: areas.gallery.width.saturating_sub(2),
        height: areas.gallery.height.saturating_sub(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_uses_bottom_three_rows() {
        let areas = three_panel_areas(Rect::new(5, 7, 101, 31));

        assert_eq!(areas.prompt.x, 5);
        assert_eq!(areas.prompt.y, 35);
        assert_eq!(areas.prompt.width, 101);
        assert_eq!(areas.prompt.height, PROMPT_HEIGHT);
        assert_eq!(areas.context.height, 28);
        assert_eq!(areas.gallery.height, 28);
        assert_eq!(areas.info.height, 28);
    }

    #[test]
    fn prompt_clamps_to_short_terminal_height() {
        let areas = three_panel_areas(Rect::new(0, 0, 80, 2));

        assert_eq!(areas.context.height, 0);
        assert_eq!(areas.prompt.y, 0);
        assert_eq!(areas.prompt.height, 2);
    }

    #[test]
    fn panels_follow_one_three_one_ratio_and_fill_width() {
        let areas = three_panel_areas(Rect::new(0, 0, 101, 20));

        assert_eq!(areas.context.width, 20);
        assert_eq!(areas.gallery.width, 60);
        assert_eq!(areas.info.width, 21);
        assert_eq!(
            areas.context.width + areas.gallery.width + areas.info.width,
            101
        );
        assert_eq!(areas.gallery.x, areas.context.width);
        assert_eq!(areas.info.x, areas.context.width + areas.gallery.width);
    }

    #[test]
    fn gallery_inner_size_matches_gallery_panel_inner() {
        let term_size = Size::new(100, 30);
        let areas = three_panel_areas(Rect::new(0, 0, term_size.width, term_size.height));
        let size = gallery_inner_size(term_size);

        assert_eq!(size.width, areas.gallery.width - 2);
        assert_eq!(size.height, areas.gallery.height.saturating_sub(2));
    }
}
