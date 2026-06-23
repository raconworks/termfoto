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

    /// Browser status bar: filename, index, total, key hints
    pub fn browser_status(&self, name: &str, selected: usize, total: usize) -> String {
        match self {
            Lang::Zh => format!(
                " {} [{}/{}]  ←→↑↓ 导航  PgUp/PgDown/Space翻页  Home/End首尾  Enter全屏  /搜索  L切换语言  q退出",
                name, selected, total
            ),
            Lang::En => format!(
                " {} [{}/{}]  ←→↑↓ Nav  PgUp/PgDown/Space Page  Home/End  Enter view  /search  L lang  q quit",
                name, selected, total
            ),
        }
    }

    /// Search bar hint when there are matches
    pub fn search_hint_matches(&self, current: usize, total: usize) -> String {
        match self {
            Lang::Zh => format!(
                " [{}/{} matches]  Tab/Shift+Tab切换  Enter全屏  Esc取消",
                current, total
            ),
            Lang::En => format!(
                " [{}/{} matches]  Tab/Shift+Tab cycle  Enter view  Esc cancel",
                current, total
            ),
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

    /// Fullscreen status bar: filename, index, total, loading suffix
    pub fn preview_status(
        &self,
        name: &str,
        selected: usize,
        total: usize,
        status: &str,
    ) -> String {
        match self {
            Lang::Zh => format!(
                " {} [{}/{}]  +/-缩放 hjkl平移 0重置  ← → 切换  Enter/Esc/q 返回  L语言{}",
                name, selected, total, status
            ),
            Lang::En => format!(
                " {} [{}/{}]  +/- zoom hjkl pan 0 reset  ← → prev/next  Enter/Esc/q back  L lang{}",
                name, selected, total, status
            ),
        }
    }

    /// Loading indicator text
    pub fn loading_text(&self) -> &'static str {
        match self {
            Lang::Zh => " ⏳ 加载中...",
            Lang::En => " ⏳ loading...",
        }
    }

    // ---- Info panel labels ----
    pub fn label_file(&self) -> &'static str {
        match self {
            Lang::Zh => "文件",
            Lang::En => "File",
        }
    }
    pub fn label_dims(&self) -> &'static str {
        match self {
            Lang::Zh => "像素",
            Lang::En => "Dimensions",
        }
    }
    pub fn label_size(&self) -> &'static str {
        match self {
            Lang::Zh => "大小",
            Lang::En => "Size",
        }
    }
    pub fn label_type(&self) -> &'static str {
        match self {
            Lang::Zh => "格式",
            Lang::En => "Type",
        }
    }
    pub fn label_path(&self) -> &'static str {
        match self {
            Lang::Zh => "路径",
            Lang::En => "Path",
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
            assert!(!lang.browser_status("test.png", 1, 10).is_empty());
            assert!(!lang.search_hint_matches(1, 5).is_empty());
            assert!(!lang.search_hint_empty().is_empty());
            assert!(!lang.search_hint_none().is_empty());
            assert!(!lang.preview_status("test.png", 1, 10, "").is_empty());
            assert!(!lang.loading_text().is_empty());
            assert!(!lang.label_file().is_empty());
            assert!(!lang.label_dims().is_empty());
            assert!(!lang.label_size().is_empty());
            assert!(!lang.label_type().is_empty());
            assert!(!lang.label_path().is_empty());
        }
    }

    #[test]
    fn test_zh_en_strings_differ() {
        assert_ne!(
            Lang::Zh.browser_status("a", 1, 5),
            Lang::En.browser_status("a", 1, 5)
        );
        assert_ne!(
            Lang::Zh.search_hint_matches(1, 5),
            Lang::En.search_hint_matches(1, 5)
        );
        assert_ne!(Lang::Zh.search_hint_empty(), Lang::En.search_hint_empty());
        assert_ne!(Lang::Zh.search_hint_none(), Lang::En.search_hint_none());
        assert_ne!(
            Lang::Zh.preview_status("a", 1, 5, ""),
            Lang::En.preview_status("a", 1, 5, "")
        );
        assert_ne!(Lang::Zh.loading_text(), Lang::En.loading_text());
        assert_ne!(Lang::Zh.label_file(), Lang::En.label_file());
        assert_ne!(Lang::Zh.label_dims(), Lang::En.label_dims());
        assert_ne!(Lang::Zh.label_size(), Lang::En.label_size());
        assert_ne!(Lang::Zh.label_type(), Lang::En.label_type());
        assert_ne!(Lang::Zh.label_path(), Lang::En.label_path());
    }
}
