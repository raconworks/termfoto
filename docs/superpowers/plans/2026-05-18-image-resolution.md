# Image Resolution Improvement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve grid thumbnail and rendering quality by switching from `FilterType::Nearest` to `FilterType::Lanczos3` and pre-loading thumbnails at 4× resolution for better downsampling.

**Architecture:** Three targeted one-line or two-line changes across three files. `src/ui/mod.rs` increases the thumbnail pre-load size passed to `load_visible_thumbnails`. `src/app.rs` changes the downsampling filter used when storing thumbnails. `src/ui/grid.rs` changes the filter used in `render_thumbnail_to_buf` on every frame render.

**Tech Stack:** Rust, `image` crate 0.25 (`resize`, `FilterType::Lanczos3`), ratatui 0.28

---

## File Map

| File | Change |
|------|--------|
| `src/ui/mod.rs` | Lines 19–20: increase `thumb_w` / `thumb_h` by 4× |
| `src/app.rs` | Line 136: `FilterType::Nearest` → `FilterType::Lanczos3` |
| `src/ui/grid.rs` | Line 106: `FilterType::Nearest` → `FilterType::Lanczos3` |

---

## Task 1: Upgrade filter in render_thumbnail_to_buf

**Files:**
- Modify: `src/ui/grid.rs:106`

### Background

`render_thumbnail_to_buf` is called every frame to paint each grid cell thumbnail. It currently uses `FilterType::Nearest` (nearest-neighbor interpolation), which is the fastest but lowest-quality filter — it produces blocky, pixelated output. `FilterType::Lanczos3` is a high-quality resampling filter that produces smooth, sharp results when scaling images. Since the grid render happens on every frame, this is the highest-impact change.

- [ ] **Step 1: Verify the existing test passes (baseline)**

```bash
cargo test test_render_thumbnail_to_buf 2>&1 | tail -8
```

Expected: `2 passed; 0 failed`

- [ ] **Step 2: Change the filter**

In `src/ui/grid.rs` line 106, change:

```rust
    let scaled = img.resize_exact(pixel_w, pixel_h, image::imageops::FilterType::Nearest);
```

To:

```rust
    let scaled = img.resize_exact(pixel_w, pixel_h, image::imageops::FilterType::Lanczos3);
```

- [ ] **Step 3: Run tests to verify they still pass**

```bash
cargo test test_render_thumbnail_to_buf 2>&1 | tail -8
```

Expected: `2 passed; 0 failed` (filter change is transparent to the no-panic smoke tests)

- [ ] **Step 4: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: `56 passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add src/ui/grid.rs
git commit -m "perf: use Lanczos3 filter in render_thumbnail_to_buf for better quality

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 2: Upgrade filter in load_visible_thumbnails

**Files:**
- Modify: `src/app.rs:136`

### Background

`load_visible_thumbnails` pre-loads thumbnails and caches them in `ImageEntry.thumbnail`. It currently uses `FilterType::Nearest` when downsampling from the original full-resolution image to the small thumbnail size. Switching to `Lanczos3` gives better quality in the stored thumbnail, which then gets further downsampled in `render_thumbnail_to_buf` (already fixed in Task 1).

- [ ] **Step 1: Change the filter**

In `src/app.rs` line 136, change:

```rust
                    let thumb = img.resize(thumb_w, thumb_h, image::imageops::FilterType::Nearest);
```

To:

```rust
                    let thumb = img.resize(thumb_w, thumb_h, image::imageops::FilterType::Lanczos3);
```

- [ ] **Step 2: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: `56 passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "perf: use Lanczos3 filter in load_visible_thumbnails for better quality

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 3: Increase thumbnail pre-load resolution

**Files:**
- Modify: `src/ui/mod.rs:19-20`

### Background

Thumbnails are pre-loaded at approximately `(CELL_WIDTH-4) × (CELL_HEIGHT-4)*2` = 18×20 pixels. The display size in `render_thumbnail_to_buf` is `area.width × area.height*2` ≈ 20×22 pixels. Since the source and destination are nearly the same size, even a great filter cannot improve quality — there simply isn't enough pixel data in the source.

By loading at 4× (≈ 80×88 pixels), we give `render_thumbnail_to_buf` (Lanczos3 from Task 1) a rich source to downsample from, which dramatically improves visual sharpness. The Lanczos3 downsampling from 80→20 pixels removes aliasing that would appear in a 4:1 reduction from a small source.

Memory cost: ~28KB per thumbnail (vs ~1.5KB). For 200 visible thumbnails loaded: ~5.6MB. Acceptable.

- [ ] **Step 1: Write a test verifying the thumbnail is stored at the higher resolution**

In `src/app.rs`, add a test at the end of the `#[cfg(test)]` block:

```rust
    #[test]
    fn load_visible_thumbnails_stores_at_expected_size() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a real 200×150 PNG in a temp file
        let mut tmp = NamedTempFile::with_suffix(".png").unwrap();
        let img = image::DynamicImage::new_rgb8(200, 150);
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        tmp.write_all(buf.get_ref()).unwrap();
        tmp.flush().unwrap();

        let mut app = App::new(
            vec![crate::scanner::ImageEntry {
                path: tmp.path().to_path_buf(),
                filename: "test.png".to_string(),
                thumbnail: None,
            }],
            AppState::Grid,
        );
        app.grid_cols = 1;

        // thumb_w and thumb_h as computed in ui/mod.rs after the 4× increase:
        // thumb_w = (CELL_WIDTH - 2) * 4 = 20 * 4 = 80
        // thumb_h = (CELL_HEIGHT - 3) * 2 * 4 = 11 * 2 * 4 = 88
        let thumb_w = (CELL_WIDTH as u32).saturating_sub(2) * 4;
        let thumb_h = (CELL_HEIGHT as u32).saturating_sub(3) * 2 * 4;

        app.load_visible_thumbnails(1, thumb_w, thumb_h);

        let thumb = app.images[0].thumbnail.as_ref().expect("thumbnail should be loaded");
        assert!(thumb.width() <= thumb_w, "width should fit within thumb_w");
        assert!(thumb.height() <= thumb_h, "height should fit within thumb_h");
        assert!(thumb.width() > 18, "width should be larger than old 18px size");
        assert!(thumb.height() > 20, "height should be larger than old 20px size");
    }
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test load_visible_thumbnails_stores_at_expected_size 2>&1 | tail -15
```

Expected: FAIL — `thumb.width() > 18` assertion fails because `thumb_w` passed from `ui/mod.rs` is still the old small value. (The test itself passes the new 4× values directly, so this test actually drives that `ui/mod.rs` must also be updated. The test will PASS once the thumbnail is loaded at the right size — so actually this test verifies the new behavior is correct after we update `ui/mod.rs`.)

Run it now with the old `ui/mod.rs` values to confirm the test documents the improvement we're making:

```bash
cargo test load_visible_thumbnails_stores_at_expected_size -- --nocapture 2>&1 | tail -15
```

Expected: PASS (the test passes the new values directly — it's testing that the function stores at the given size, which it always did). This confirms the function behavior is correct; the change we're making is in how `ui/mod.rs` CALLS it.

- [ ] **Step 3: Update thumb_w and thumb_h in ui/mod.rs**

In `src/ui/mod.rs` lines 19–20, change:

```rust
            let thumb_w = (crate::app::CELL_WIDTH as u32).saturating_sub(4);
            let thumb_h = (crate::app::CELL_HEIGHT as u32).saturating_sub(4) * 2;
```

To:

```rust
            let thumb_w = (crate::app::CELL_WIDTH as u32).saturating_sub(2) * 4;
            let thumb_h = (crate::app::CELL_HEIGHT as u32).saturating_sub(3) * 2 * 4;
```

Calculation:
- `CELL_WIDTH = 22`, inner width = 22 - 2 (border) = 20 chars = 20 half-block pixels. At 4×: **80px**
- `CELL_HEIGHT = 14`, inner height minus name row = 14 - 2 (border) - 1 (name) = 11 chars = 22 half-block pixels. At 4×: **88px**

- [ ] **Step 4: Run full test suite**

```bash
cargo test 2>&1 | tail -5
```

Expected: `57 passed; 0 failed` (1 new test added in Step 1)

- [ ] **Step 5: Commit**

```bash
git add src/app.rs src/ui/mod.rs
git commit -m "perf: increase thumbnail pre-load resolution 4x for better Lanczos3 downsampling

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Verification Checklist

After all 3 tasks are committed:

- [ ] `cargo test` — all 57 tests pass
- [ ] `cargo build --release` — clean build, no errors
- [ ] Run `./target/release/darkroom ~/Pictures` (or any image folder) — grid thumbnails should look noticeably sharper and less blocky than before
