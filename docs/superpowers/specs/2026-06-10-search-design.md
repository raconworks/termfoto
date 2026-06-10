# 浏览器图片搜索功能设计

## 概述

在浏览器界面添加增量搜索功能，以 `/` 或 `\` 触发，输入字符即时跳转并高亮最佳匹配结果。

## 交互设计

### 触发与退出

| 操作 | 行为 |
|------|------|
| `/` 或 `\` | 进入搜索模式，保存当前光标位置 |
| `Esc` | 退出搜索，恢复搜索前光标位置 |

### 搜索行为

- **增量匹配**：每输入一个字符，立即更新匹配列表，自动跳到评分最高的匹配项
- **模糊匹配**：查询字符在文件名中按顺序出现即可（中间可跳过），大小写不敏感
- **评分规则**：连续匹配加分、匹配位置靠前加分、字符间隔短加分
- **匹配项切换**：`Tab` 跳到下一个匹配项，`Shift+Tab` 跳到上一个匹配项（循环）
- **无匹配**：query 文字变红，显示 `[0/0]`

### 搜索栏 UI

显示在浏览器底部，替代原有状态栏（1 行高度）：

```
/photo_4█ [3/12 matches]  Tab/Shift+Tab切换  Esc取消
```

- `/`：触发键（与用户按键一致，`\` 触发展示 `\`）
- `photo_4`：用户输入的 query 文本
- `█`：闪烁光标（反色字符，帧计数取模实现闪烁）
- `[3/12 matches]`：当前匹配项序号 / 总匹配数
- `Tab/Shift+Tab切换  Esc取消`：操作提示

### 高亮

- 当前匹配的 cell 边框变**黄色**
- 文件名中匹配字符**黄色高亮**（逐个字符比对）

## 架构设计

### 新增文件：`src/ui/search.rs`

#### `SearchState`

```rust
pub struct SearchState {
    pub query: String,
    pub matches: Vec<usize>,      // 匹配的图片索引，按评分降序
    pub match_idx: usize,         // 当前高亮位置（Tab 切换用）
    pub saved_selected: usize,    // 搜索前光标位置
    pub trigger_char: char,       // '/' 或 '\'
}

pub enum SearchAction {
    Continue,
    JumpTo(usize),
    Cancel,
}
```

**方法**：
- `new(current_selected: usize, trigger: char) -> Self`
- `handle_key(code, modifiers, images) -> SearchAction`：分发按键，更新匹配并返回动作
- `update_matches(images)`：对 `ImageEntry.filename` 执行模糊匹配

#### `SearchBar` Widget

渲染搜索栏。`trigger_char` 决定 `/` 或 `\` 前缀。query 为空时显示光标，query 非空时光标闪烁。

### 修改文件

| 文件 | 改动 |
|------|------|
| `src/ui/mod.rs` | `pub mod search;` |
| `src/ui/browser.rs` | 搜索模式下底部渲染 `SearchBar`（替换状态栏）；匹配项边框黄色 + 文件名匹配字符黄色高亮 |
| `src/app.rs` | 新增 `search: Option<SearchState>` 字段；`handle_key` 在浏览器模式下新增搜索分支 |

### 搜索模式按键处理（`handle_key` 内部新增函数）

- `Char(c)` → 追加到 query，更新匹配，跳到最佳匹配
- `Backspace` → 删除最后一个字符，更新匹配
- `Tab` → `match_idx = (match_idx + 1) % matches.len()`，跳到 `matches[match_idx]`
- `BackTab`（Shift+Tab）→ `match_idx = (match_idx - 1 + n) % n`，跳到对应位置
- `Esc` → 清空 search，恢复 `saved_selected`
- 其他键 → 忽略（不退出搜索模式）

## 模糊匹配算法

```
对于每个 ImageEntry.filename:
  1. 将 query 和 filename 都转为小写
  2. 贪心扫描：在 filename 中按顺序查找 query 的每个字符
  3. 若全部找到 → 匹配成功，计算评分
  4. 评分 = Σ(连续匹配加分 + 靠前加分 - 间隔惩罚)
  5. 按评分降序排列，存入 matches
```

## 测试要点

- `SearchState::update_matches` 模糊匹配正确性（子串、跳跃、大小写）
- `SearchState::handle_key` Tab/Shift+Tab 循环切换
- `SearchState::handle_key` Esc 返回 Cancel
- `SearchState::new` 保存 saved_selected
- 无匹配时 query 变红
- 浏览器渲染：匹配项边框黄色、文件名匹配字符高亮
