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

    pub fn title_context(&self) -> &'static str {
        match self {
            Lang::Zh => "上级目录",
            Lang::En => "Context",
        }
    }

    pub fn title_gallery(&self) -> &'static str {
        match self {
            Lang::Zh => "图片浏览",
            Lang::En => "Gallery",
        }
    }

    pub fn title_favorites(&self) -> &'static str {
        match self {
            Lang::Zh => "收藏",
            Lang::En => "Favorites",
        }
    }

    pub fn title_info(&self) -> &'static str {
        match self {
            Lang::Zh => "文件信息",
            Lang::En => "Info",
        }
    }

    pub fn empty_folder_context(&self) -> &'static str {
        match self {
            Lang::Zh => "没有子文件夹",
            Lang::En => "No folders",
        }
    }

    pub fn browser_prompt_lines(
        &self,
        name: &str,
        selected: usize,
        total: usize,
        sort_label: &str,
    ) -> Vec<String> {
        match self {
            Lang::Zh => vec![
                format!(
                    " 文件     {} [{}/{}]   排列 {}",
                    name, selected, total, sort_label
                ),
                " 导航     ←→↑↓ 导航   PgUp/PgDown/Space 翻页   Home/End 首尾".to_string(),
                " 操作     Enter 全屏   Tab 切换面板   / 搜索   r 重命名   f 收藏   F 收藏视图   s 排列   L 切换语言   q 退出"
                    .to_string(),
            ],
            Lang::En => vec![
                format!(
                    " File     {} [{}/{}]   Sort {}",
                    name, selected, total, sort_label
                ),
                " Move     ←→↑↓ Nav   PgUp/PgDown/Space Page   Home/End First/Last".to_string(),
                " Action   Enter View   Tab Focus   / Search   r Rename   f Favorite   F Favorites   s Sort   L Language   q Quit"
                    .to_string(),
            ],
        }
    }

    pub fn favorites_prompt_lines(&self, name: &str, selected: usize, total: usize) -> Vec<String> {
        match self {
            Lang::Zh => vec![
                format!(" 收藏视图 {} [{}/{}]", name, selected, total),
                " 导航     ←→↑↓ 导航   PgUp/PgDown/Space 翻页   Home/End 首尾".to_string(),
                " 操作     Enter 全屏   / 搜索   f 取消收藏   F 返回图库   L 切换语言   q 退出"
                    .to_string(),
            ],
            Lang::En => vec![
                format!(" Favorites {} [{}/{}]", name, selected, total),
                " Move     ←→↑↓ Nav   PgUp/PgDown/Space Page   Home/End First/Last".to_string(),
                " Action   Enter View   / Search   f Unfavorite   F Gallery   L Language   q Quit"
                    .to_string(),
            ],
        }
    }

    pub fn context_prompt_lines(
        &self,
        name: &str,
        selected: usize,
        total: usize,
        sort_label: &str,
    ) -> Vec<String> {
        match self {
            Lang::Zh => vec![
                format!(
                    " 文件夹   {} [{}/{}]   排列 {}",
                    name, selected, total, sort_label
                ),
                " 导航     ↑↓ 选择   Home/End 首尾   ← 上一级".to_string(),
                " 操作     →/Enter 进入文件夹   Tab 切换面板   f 收藏   F 收藏视图   s 排列   L 切换语言   q 退出"
                    .to_string(),
            ],
            Lang::En => vec![
                format!(
                    " Folder   {} [{}/{}]   Sort {}",
                    name, selected, total, sort_label
                ),
                " Move     ↑↓ Select   Home/End First/Last   ← Parent".to_string(),
                " Action   →/Enter Open Folder   Tab Focus   f Favorite   F Favorites   s Sort   L Language   q Quit"
                    .to_string(),
            ],
        }
    }

    pub fn directory_error(&self) -> &'static str {
        match self {
            Lang::Zh => "无法读取目录",
            Lang::En => "Could not read directory",
        }
    }

    pub fn status_prompt_line(&self, message: &str) -> String {
        match self {
            Lang::Zh => format!(" 状态     {}", message),
            Lang::En => format!(" Status   {}", message),
        }
    }

    pub fn search_prompt_lines(
        &self,
        current: usize,
        total: usize,
        has_query: bool,
    ) -> Vec<String> {
        match self {
            Lang::Zh => {
                let matches = if total > 0 {
                    format!(" 匹配: {}/{}", current, total)
                } else if has_query {
                    " 匹配: 0/0".to_string()
                } else {
                    " 输入文件名进行搜索".to_string()
                };
                vec![
                    String::new(),
                    format!(" 状态     {}", matches.trim_start()),
                    " 操作     Tab/Shift+Tab 切换   Enter 全屏   Esc 取消".to_string(),
                ]
            }
            Lang::En => {
                let matches = if total > 0 {
                    format!(" Matches: {}/{}", current, total)
                } else if has_query {
                    " Matches: 0/0".to_string()
                } else {
                    " Type to search filenames".to_string()
                };
                vec![
                    String::new(),
                    format!(" Status   {}", matches.trim_start()),
                    " Action   Tab/Shift+Tab Cycle   Enter View   Esc Cancel".to_string(),
                ]
            }
        }
    }

    pub fn fullscreen_prompt_lines(
        &self,
        name: &str,
        selected: usize,
        total: usize,
        status: &str,
        favorites_view: bool,
    ) -> Vec<String> {
        let switch_label = match (self, favorites_view) {
            (Lang::Zh, true) => "F 返回图库",
            (Lang::Zh, false) => "F 收藏视图",
            (Lang::En, true) => "F Gallery",
            (Lang::En, false) => "F Favorites",
        };
        match self {
            Lang::Zh => vec![
                format!(" 文件     {} [{}/{}]{}", name, selected, total, status),
                " 视图     +/- 缩放   0 重置   hjkl 平移".to_string(),
                format!(
                    " 操作     ← → 切换图片   r 重命名   f 收藏   {}   Enter/Esc/q 返回   L 语言",
                    switch_label
                ),
            ],
            Lang::En => vec![
                format!(" File     {} [{}/{}]{}", name, selected, total, status),
                " View     +/- Zoom   0 Reset   hjkl Pan".to_string(),
                format!(
                    " Action   ← → Prev/Next   r Rename   f Favorite   {}   Enter/Esc/q Back   L Language",
                    switch_label
                ),
            ],
        }
    }

    pub fn rename_prompt_lines(
        &self,
        original_name: &str,
        input: &str,
        message: Option<&str>,
    ) -> Vec<String> {
        match self {
            Lang::Zh => vec![
                format!(" 重命名   {} -> [{}]", original_name, input),
                message
                    .map(|message| format!(" 状态     {}", message))
                    .unwrap_or_default(),
                " 操作     Enter 保存   Esc 取消".to_string(),
            ],
            Lang::En => vec![
                format!(" Rename   {} -> [{}]", original_name, input),
                message
                    .map(|message| format!(" Status   {}", message))
                    .unwrap_or_default(),
                " Action   Enter Save   Esc Cancel".to_string(),
            ],
        }
    }

    pub fn rename_overwrite_prompt_lines(
        &self,
        original_name: &str,
        target_name: &str,
    ) -> Vec<String> {
        match self {
            Lang::Zh => vec![
                format!(" 重命名   {} -> {}", original_name, target_name),
                " 状态     目标已存在，覆盖？".to_string(),
                " 操作     y 确认 / n 取消".to_string(),
            ],
            Lang::En => vec![
                format!(" Rename   {} -> {}", original_name, target_name),
                " Status   Target exists. Overwrite?".to_string(),
                " Action   y Yes / n No".to_string(),
            ],
        }
    }

    pub fn rename_no_image(&self) -> &'static str {
        match self {
            Lang::Zh => "没有可重命名的图片",
            Lang::En => "No image to rename",
        }
    }

    pub fn rename_empty_name(&self) -> &'static str {
        match self {
            Lang::Zh => "名称不能为空",
            Lang::En => "Name cannot be empty",
        }
    }

    pub fn rename_invalid_name(&self) -> &'static str {
        match self {
            Lang::Zh => "名称不能包含路径分隔符",
            Lang::En => "Name cannot contain path separators",
        }
    }

    pub fn rename_unchanged(&self) -> &'static str {
        match self {
            Lang::Zh => "名称未改变",
            Lang::En => "Name unchanged",
        }
    }

    pub fn rename_cancelled(&self) -> &'static str {
        match self {
            Lang::Zh => "已取消重命名",
            Lang::En => "Rename cancelled",
        }
    }

    pub fn rename_success(&self, name: &str) -> String {
        match self {
            Lang::Zh => format!("已重命名为 {}", name),
            Lang::En => format!("Renamed to {}", name),
        }
    }

    pub fn rename_failed(&self, error: &str) -> String {
        match self {
            Lang::Zh => format!("重命名失败: {}", error),
            Lang::En => format!("Rename failed: {}", error),
        }
    }

    pub fn favorite_badge(&self) -> &'static str {
        match self {
            Lang::Zh => "收藏",
            Lang::En => "Favorite",
        }
    }

    pub fn favorite_added(&self) -> &'static str {
        match self {
            Lang::Zh => "已收藏",
            Lang::En => "Favorited",
        }
    }

    pub fn favorite_removed(&self) -> &'static str {
        match self {
            Lang::Zh => "已取消收藏",
            Lang::En => "Unfavorited",
        }
    }

    pub fn favorite_no_image(&self) -> &'static str {
        match self {
            Lang::Zh => "没有可收藏图片",
            Lang::En => "No image to favorite",
        }
    }

    pub fn favorite_none(&self) -> &'static str {
        match self {
            Lang::Zh => "没有收藏",
            Lang::En => "No favorites",
        }
    }

    pub fn favorite_save_failed(&self, error: &str) -> String {
        match self {
            Lang::Zh => format!("收藏保存失败: {}", error),
            Lang::En => format!("Favorite save failed: {}", error),
        }
    }

    pub fn favorite_missing_skipped(&self, count: usize) -> String {
        match self {
            Lang::Zh => format!("已跳过 {} 个缺失收藏文件", count),
            Lang::En => format!("Skipped {} missing favorite file(s)", count),
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
    pub fn label_modified(&self) -> &'static str {
        match self {
            Lang::Zh => "修改时间",
            Lang::En => "Modified",
        }
    }
    pub fn label_created(&self) -> &'static str {
        match self {
            Lang::Zh => "创建时间",
            Lang::En => "Created",
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
            assert!(!lang.title_context().is_empty());
            assert!(!lang.title_gallery().is_empty());
            assert!(!lang.title_favorites().is_empty());
            assert!(!lang.title_info().is_empty());
            assert!(!lang.empty_folder_context().is_empty());
            assert_eq!(
                lang.browser_prompt_lines("test.png", 1, 10, "Name").len(),
                3
            );
            assert_eq!(lang.context_prompt_lines("photos", 1, 3, "Name").len(), 3);
            assert_eq!(lang.search_prompt_lines(1, 5, true).len(), 3);
            assert_eq!(
                lang.fullscreen_prompt_lines("test.png", 1, 10, "", false)
                    .len(),
                3
            );
            assert_eq!(lang.favorites_prompt_lines("test.png", 1, 10).len(), 3);
            assert_eq!(lang.rename_prompt_lines("old.png", "new", None).len(), 3);
            assert_eq!(
                lang.rename_overwrite_prompt_lines("old.png", "new.png")
                    .len(),
                3
            );
            assert!(!lang.loading_text().is_empty());
            assert!(!lang.directory_error().is_empty());
            assert!(!lang.status_prompt_line("message").is_empty());
            assert!(!lang.rename_no_image().is_empty());
            assert!(!lang.rename_empty_name().is_empty());
            assert!(!lang.rename_invalid_name().is_empty());
            assert!(!lang.rename_unchanged().is_empty());
            assert!(!lang.rename_cancelled().is_empty());
            assert!(!lang.rename_success("new.png").is_empty());
            assert!(!lang.rename_failed("error").is_empty());
            assert!(!lang.favorite_badge().is_empty());
            assert!(!lang.favorite_added().is_empty());
            assert!(!lang.favorite_removed().is_empty());
            assert!(!lang.favorite_no_image().is_empty());
            assert!(!lang.favorite_none().is_empty());
            assert!(!lang.favorite_save_failed("error").is_empty());
            assert!(!lang.favorite_missing_skipped(1).is_empty());
            assert!(!lang.label_file().is_empty());
            assert!(!lang.label_dims().is_empty());
            assert!(!lang.label_size().is_empty());
            assert!(!lang.label_type().is_empty());
            assert!(!lang.label_path().is_empty());
            assert!(!lang.label_modified().is_empty());
            assert!(!lang.label_created().is_empty());
        }
    }

    #[test]
    fn test_zh_en_strings_differ() {
        assert_ne!(Lang::Zh.title_context(), Lang::En.title_context());
        assert_ne!(Lang::Zh.title_gallery(), Lang::En.title_gallery());
        assert_ne!(Lang::Zh.title_favorites(), Lang::En.title_favorites());
        assert_ne!(Lang::Zh.title_info(), Lang::En.title_info());
        assert_ne!(
            Lang::Zh.empty_folder_context(),
            Lang::En.empty_folder_context()
        );
        assert_ne!(
            Lang::Zh.browser_prompt_lines("a", 1, 5, "名称"),
            Lang::En.browser_prompt_lines("a", 1, 5, "Name")
        );
        assert_ne!(
            Lang::Zh.context_prompt_lines("a", 1, 5, "名称"),
            Lang::En.context_prompt_lines("a", 1, 5, "Name")
        );
        assert_ne!(
            Lang::Zh.search_prompt_lines(1, 5, true),
            Lang::En.search_prompt_lines(1, 5, true)
        );
        assert_ne!(
            Lang::Zh.fullscreen_prompt_lines("a", 1, 5, "", false),
            Lang::En.fullscreen_prompt_lines("a", 1, 5, "", false)
        );
        assert_ne!(
            Lang::Zh.favorites_prompt_lines("a", 1, 5),
            Lang::En.favorites_prompt_lines("a", 1, 5)
        );
        assert_ne!(
            Lang::Zh.rename_prompt_lines("a.png", "b", Some("状态")),
            Lang::En.rename_prompt_lines("a.png", "b", Some("status"))
        );
        assert_ne!(
            Lang::Zh.rename_overwrite_prompt_lines("a.png", "b.png"),
            Lang::En.rename_overwrite_prompt_lines("a.png", "b.png")
        );
        assert_ne!(Lang::Zh.loading_text(), Lang::En.loading_text());
        assert_ne!(Lang::Zh.directory_error(), Lang::En.directory_error());
        assert_ne!(
            Lang::Zh.status_prompt_line("message"),
            Lang::En.status_prompt_line("message")
        );
        assert_ne!(Lang::Zh.rename_no_image(), Lang::En.rename_no_image());
        assert_ne!(Lang::Zh.rename_empty_name(), Lang::En.rename_empty_name());
        assert_ne!(
            Lang::Zh.rename_invalid_name(),
            Lang::En.rename_invalid_name()
        );
        assert_ne!(Lang::Zh.rename_unchanged(), Lang::En.rename_unchanged());
        assert_ne!(Lang::Zh.rename_cancelled(), Lang::En.rename_cancelled());
        assert_ne!(
            Lang::Zh.rename_success("b.png"),
            Lang::En.rename_success("b.png")
        );
        assert_ne!(
            Lang::Zh.rename_failed("error"),
            Lang::En.rename_failed("error")
        );
        assert_ne!(Lang::Zh.favorite_badge(), Lang::En.favorite_badge());
        assert_ne!(Lang::Zh.favorite_added(), Lang::En.favorite_added());
        assert_ne!(Lang::Zh.favorite_removed(), Lang::En.favorite_removed());
        assert_ne!(Lang::Zh.favorite_no_image(), Lang::En.favorite_no_image());
        assert_ne!(Lang::Zh.favorite_none(), Lang::En.favorite_none());
        assert_ne!(
            Lang::Zh.favorite_save_failed("error"),
            Lang::En.favorite_save_failed("error")
        );
        assert_ne!(
            Lang::Zh.favorite_missing_skipped(1),
            Lang::En.favorite_missing_skipped(1)
        );
        assert_ne!(Lang::Zh.label_file(), Lang::En.label_file());
        assert_ne!(Lang::Zh.label_dims(), Lang::En.label_dims());
        assert_ne!(Lang::Zh.label_size(), Lang::En.label_size());
        assert_ne!(Lang::Zh.label_type(), Lang::En.label_type());
        assert_ne!(Lang::Zh.label_path(), Lang::En.label_path());
        assert_ne!(Lang::Zh.label_modified(), Lang::En.label_modified());
        assert_ne!(Lang::Zh.label_created(), Lang::En.label_created());
    }
}
