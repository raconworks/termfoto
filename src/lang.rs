/// Language for UI text. Detected from $LANG at startup, toggled by L key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    /// Detect language from $LANG environment variable.
    /// zh_CN, zh_TW, zh_SG, etc. → Zh; everything else → En.
    pub fn detect() -> Self {
        match std::env::var("LANG") {
            Ok(s) if s.starts_with("zh_") => Lang::Zh,
            _ => Lang::En,
        }
    }

    /// Toggle between Chinese and English.
    pub fn toggle(&mut self) {
        *self = match self {
            Lang::Zh => Lang::En,
            Lang::En => Lang::Zh,
        };
    }

    /// Browser status bar format: filename, index, total, key hints
    pub fn browser_status_fmt(&self) -> &'static str {
        match self {
            Lang::Zh => " {} [{}/{}]  ←→↑↓ 导航  PgUp/PgDown/Space翻页  Home/End首尾  Enter全屏  /搜索  q退出",
            Lang::En => " {} [{}/{}]  ←→↑↓ Nav  PgUp/PgDown/Space Page  Home/End  Enter view  /search  q quit",
        }
    }

    /// Search bar hint when there are matches
    pub fn search_hint_matches(&self) -> &'static str {
        match self {
            Lang::Zh => " [{}/{} matches]  Tab/Shift+Tab切换  Enter全屏  Esc取消",
            Lang::En => " [{}/{} matches]  Tab/Shift+Tab cycle  Enter view  Esc cancel",
        }
    }

    /// Search bar hint when query is empty
    pub fn search_hint_empty(&self) -> &'static str {
        match self {
            Lang::Zh => " Tab/Shift+Tab切换  Enter全屏  Esc取消",
            Lang::En => " Tab/Shift+Tab cycle  Enter view  Esc cancel",
        }
    }

    /// Search bar hint when no matches found
    pub fn search_hint_none(&self) -> &'static str {
        match self {
            Lang::Zh => " [0/0]  Tab/Shift+Tab切换  Enter全屏  Esc取消",
            Lang::En => " [0/0]  Tab/Shift+Tab cycle  Enter view  Esc cancel",
        }
    }

    /// Fullscreen status bar format: filename, index, total, loading suffix
    pub fn preview_status_fmt(&self) -> &'static str {
        match self {
            Lang::Zh => " {} [{}/{}]  原图尺寸  ← → 切换  Enter/Esc/q 返回{}",
            Lang::En => " {} [{}/{}]  original size  ← → prev/next  Enter/Esc/q back{}",
        }
    }

    /// Loading indicator text
    pub fn loading_text(&self) -> &'static str {
        match self {
            Lang::Zh => " ⏳ 加载中...",
            Lang::En => " ⏳ loading...",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_zh_to_en() {
        let mut lang = Lang::Zh;
        lang.toggle();
        assert_eq!(lang, Lang::En);
    }

    #[test]
    fn test_toggle_en_to_zh() {
        let mut lang = Lang::En;
        lang.toggle();
        assert_eq!(lang, Lang::Zh);
    }

    #[test]
    fn test_toggle_roundtrip() {
        let mut lang = Lang::Zh;
        lang.toggle();
        lang.toggle();
        assert_eq!(lang, Lang::Zh);
    }

    #[test]
    fn test_all_methods_return_non_empty() {
        for lang in [Lang::Zh, Lang::En] {
            assert!(!lang.browser_status_fmt().is_empty());
            assert!(!lang.search_hint_matches().is_empty());
            assert!(!lang.search_hint_empty().is_empty());
            assert!(!lang.search_hint_none().is_empty());
            assert!(!lang.preview_status_fmt().is_empty());
            assert!(!lang.loading_text().is_empty());
        }
    }

    #[test]
    fn test_zh_en_strings_differ() {
        assert_ne!(Lang::Zh.browser_status_fmt(), Lang::En.browser_status_fmt());
        assert_ne!(Lang::Zh.search_hint_matches(), Lang::En.search_hint_matches());
        assert_ne!(Lang::Zh.search_hint_empty(), Lang::En.search_hint_empty());
        assert_ne!(Lang::Zh.search_hint_none(), Lang::En.search_hint_none());
        assert_ne!(Lang::Zh.preview_status_fmt(), Lang::En.preview_status_fmt());
        assert_ne!(Lang::Zh.loading_text(), Lang::En.loading_text());
    }
}
