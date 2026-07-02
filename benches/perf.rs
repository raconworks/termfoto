use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use image::{ImageBuffer, Rgba, RgbaImage};
use ratatui::layout::Size;
use ratatui_image::picker::Picker;
use tempfile::TempDir;
use termfoto::app::bench::{process_thumbnail_request_for_bench, FullscreenRenderBench};
use termfoto::scanner::{scan_directory, ImageEntry};
use termfoto::ui::search::search_matches_for_bench;

const SCAN_FILE_COUNT: usize = 10_000;
const SEARCH_IMAGE_COUNT: usize = 10_000;
const FOUR_K_W: u32 = 3840;
const FOUR_K_H: u32 = 2160;

fn bench_scan_directory(c: &mut Criterion) {
    let fixture = scan_fixture(SCAN_FILE_COUNT);

    c.bench_function("scan_directory_10000", |b| {
        b.iter(|| {
            let entries = scan_directory(black_box(fixture.path())).unwrap();
            black_box(entries.len())
        });
    });
}

fn bench_search(c: &mut Criterion) {
    let images = search_fixture(SEARCH_IMAGE_COUNT);

    c.bench_function("search_10000_ascii", |b| {
        b.iter(|| {
            black_box(search_matches_for_bench(
                black_box("img999"),
                black_box(&images),
            ))
        });
    });
}

fn bench_thumbnail(c: &mut Criterion) {
    let fixture = png_fixture("thumbnail_4k.png");
    let path = fixture.path().join("thumbnail_4k.png");
    let picker = Picker::halfblocks();

    c.bench_function("thumbnail_4k_png", |b| {
        b.iter(|| {
            black_box(process_thumbnail_request_for_bench(
                black_box(&picker),
                black_box(&path),
                24,
                12,
            ))
        });
    });
}

fn bench_fullscreen_render(c: &mut Criterion) {
    let image = Arc::new(gradient_rgba(FOUR_K_W, FOUR_K_H));
    let picker = Picker::halfblocks();
    let mut renderer = FullscreenRenderBench::new(picker);

    c.bench_function("fullscreen_render_4k_final", |b| {
        b.iter(|| {
            black_box(renderer.render_final(
                black_box(Arc::clone(&image)),
                black_box(Size::new(120, 40)),
                black_box(1.0),
            ))
        });
    });
}

fn scan_fixture(count: usize) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for idx in 0..count {
        let ext = match idx % 4 {
            0 => "png",
            1 => "JPG",
            2 => "webp",
            _ => "txt",
        };
        fs::write(dir.path().join(format!("image_{idx:05}.{ext}")), b"x").unwrap();
    }
    dir
}

fn search_fixture(count: usize) -> Vec<ImageEntry> {
    (0..count)
        .map(|idx| {
            let filename = format!("vacation_img{idx:05}_sunset.PNG");
            ImageEntry {
                path: PathBuf::from(&filename),
                filename,
                file_size: 0,
                modified_at: None,
            }
        })
        .collect()
}

fn png_fixture(filename: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(filename);
    gradient_rgba(FOUR_K_W, FOUR_K_H).save(path).unwrap();
    dir
}

fn gradient_rgba(width: u32, height: u32) -> RgbaImage {
    ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    })
}

criterion_group!(
    benches,
    bench_scan_directory,
    bench_search,
    bench_thumbnail,
    bench_fullscreen_render
);
criterion_main!(benches);
