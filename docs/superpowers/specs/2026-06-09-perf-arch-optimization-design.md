# 性能与架构优化设计

**日期**: 2026-06-09  
**状态**: 设计中  
**目标**: 消除主线程阻塞、统一加载管线、优化代码架构

## 设计哲学

本次优化遵循项目核心哲学：**效率优先，功能其次**。不引入重量级框架或过度抽象，每项改动都有明确的性能或可维护性收益。

## 改动清单

### 1. Loader 统一加载管线

**问题**: 浏览器缩略图在主线程同步调用 `image::open()` + `picker.new_protocol()`，滚动时卡 UI。全屏用后台线程，两套机制割裂。

**方案**: 扩展现有后台线程，浏览器也通过 `load_tx` 发请求，`collect_loads()` 统一收结果。

```
                    load_tx
main thread ──────► channel ──► background thread
                                  │ image::open()
                   done_rx        │ picker.new_protocol()
main thread ◄────── channel ◄─────┘
```

**改动**:
- `spawn_image_loader` 保持不变（线程内部逻辑不变）
- `populate_protocol_cache` 移除 `image::open()` + `picker.new_protocol()` 同步调用，改为 `load_tx.send(slot)`
- `collect_loads()` 扩展：收到的 protocol 若匹配浏览器模式则插入 `protocol_cache`，匹配全屏模式则设为 `fullscreen_protocol`

### 2. 有界 FIFO 缓存

**问题**: `protocol_cache: HashMap<usize, Protocol>` 无上限增长，大目录内存不受控。

**方案**: 容量上限 200 个 Protocol，超量时清掉最早的一半。HashMap 的 hashbrown 实现天然保持插入顺序，无需额外链表。

```rust
const MAX_CACHE_SIZE: usize = 200;

fn insert_cache(&mut self, idx: usize, proto: Protocol) {
    self.protocol_cache.insert(idx, proto);
    if self.protocol_cache.len() > MAX_CACHE_SIZE {
        let remove_count = MAX_CACHE_SIZE / 2;
        let keys: Vec<usize> = self.protocol_cache
            .keys()
            .take(remove_count)
            .copied()
            .collect();
        for k in keys {
            self.protocol_cache.remove(&k);
        }
    }
}
```

### 3. 相邻预取

**问题**: 滚动到新区时空白格需要等下一次事件循环才填充。

**方案**: 请求可见区域的同时，顺带请求前后各一行（8 张）图片。新增 `requested: HashSet<usize>` 去重标记，避免重复发送同一 slot 的加载请求。

```
请求范围:
┌─────────────────────┐  ← 预取：前一行（8 张）
│  可见区域（N 行）   │  ← 立即请求
└─────────────────────┘  ← 预取：后一行（8 张）
```

### 4. TermGuard — RAII 终端管理

**问题**: `main()` 中终端 init/cleanup 逻辑与错误路径交织，约 25 行，重复且易漏清理。

**方案**: 封装为 `TermGuard` 结构体，构造时 `enable_raw_mode()` + `EnterAlternateScreen`，Drop 时自动恢复。

```rust
struct TermGuard {
    stdout: io::Stdout,
}
impl TermGuard {
    fn enter() -> Result<Self> { ... }
    fn backend(&self) -> CrosstermBackend<io::Stdout> { ... }
}
impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
```

即使 `run()` 提前 `?` 返回，Drop 保证终端正确恢复。

### 5. handle_key 移入 App + 消除 take/put-back

**问题**: `handle_key` 是 `main.rs` 自由函数，需要外部传 `visible_rows`；渲染时需要 `take`/`put-back` 变通借用检查。

**方案**:
- `handle_key` 变为 `App::handle_key(&mut self, code, modifiers) -> bool`，`visible_rows` 作为 App 字段在 draw 前注入
- `PreviewView` 直接从 `self.app.fullscreen_protocol` 读取，不再需要外部 `Option<&Protocol>` 参数
- `draw()` 签名简化为 `draw(frame, app, cell_w, cell_h)`

## 涉及文件

| 文件 | 改动 |
|------|------|
| `src/main.rs` | 引入 TermGuard；移除 take/put-back；事件循环简化 |
| `src/app.rs` | handle_key 移入；collect_loads 扩展；缓存有界化；requested 去重集；visible_rows 字段 |
| `src/ui/mod.rs` | draw() 签名简化，移除 protocol 参数 |
| `src/ui/browser.rs` | populate_protocol_cache 改为异步请求 |
| `src/ui/preview.rs` | 直接从 app 读取 fullscreen_protocol |

## 不变项

- 所有功能行为不变（快捷键、布局、渲染效果）
- 后台线程仍然单线程（不引入线程池，保持轻量）
- CELL_HEIGHT 保持编译期常量
- Picker / Protocol 用法不变

## 验证

```bash
cargo test                    # 全部测试通过
cargo clippy                  # 无 lint 警告
cargo build --release         # release 编译通过
cargo run -- <测试目录>       # 浏览器滚动不卡顿，全屏切换流畅
```
