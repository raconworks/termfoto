use super::process::skipped_load_result;
use super::*;

use std::fs::File;
use std::io::BufReader;

use image::AnimationDecoder;

pub(in crate::app) fn frame_delay(delay: image::Delay) -> Duration {
    let (numer, denom) = delay.numer_denom_ms();
    if denom == 0 {
        return DEFAULT_FRAME_DELAY;
    }
    let millis = u64::from(numer) / u64::from(denom);
    let duration = if millis == 0 {
        DEFAULT_FRAME_DELAY
    } else {
        Duration::from_millis(millis)
    };
    duration.max(MIN_FRAME_DELAY)
}

pub(in crate::app) fn make_protocol(
    picker: &Picker,
    img: image::DynamicImage,
    size: Size,
    filter: ProtocolFilterType,
) -> Option<Protocol> {
    picker
        .new_protocol(img, size, Resize::Fit(Some(filter)))
        .ok()
}

#[cfg(test)]
pub(in crate::app) fn static_original_content(img: image::DynamicImage) -> FullscreenContent {
    static_rgba_content(img.into_rgba8())
}

pub(in crate::app) fn static_rgba_content(img: image::RgbaImage) -> FullscreenContent {
    FullscreenContent::Static(StaticContent {
        protocol: None,
        original: Arc::new(img),
    })
}

fn animation_frame_from_image_frame(
    picker: &Picker,
    frame: image::Frame,
    size: Size,
) -> Option<AnimationFrame> {
    let delay = frame_delay(frame.delay());
    let img = image::DynamicImage::ImageRgba8(frame.into_buffer());
    let protocol = make_protocol(picker, img, size, ProtocolFilterType::Nearest)?;
    Some(AnimationFrame { protocol, delay })
}

#[cfg(test)]
pub(in crate::app) fn animation_content_from_frames<I>(
    picker: &Picker,
    frames: I,
    size: Size,
) -> Option<AnimationContent>
where
    I: IntoIterator<Item = image::ImageResult<image::Frame>>,
{
    let mut animation_frames = Vec::new();
    for frame in frames {
        if animation_frames.len() == MAX_ANIMATION_FRAMES {
            return None;
        }
        let frame = frame.ok()?;
        animation_frames.push(animation_frame_from_image_frame(picker, frame, size)?);
    }

    if animation_frames.len() >= 2 {
        Some(AnimationContent {
            estimated_bytes: animation_frames_estimated_bytes(
                &animation_frames,
                picker.font_size(),
            ),
            frames: animation_frames,
            complete: true,
        })
    } else {
        None
    }
}

#[cfg(test)]
pub(in crate::app) fn try_decode_animation(
    picker: &Picker,
    path: &Path,
    size: Size,
) -> Option<AnimationContent> {
    let format = image::ImageFormat::from_path(path).ok()?;
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    match format {
        image::ImageFormat::Gif => {
            let decoder = image::codecs::gif::GifDecoder::new(reader).ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        image::ImageFormat::Png => {
            let decoder = image::codecs::png::PngDecoder::new(reader).ok()?;
            let decoder = decoder.apng().ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        image::ImageFormat::WebP => {
            let decoder = image::codecs::webp::WebPDecoder::new(reader).ok()?;
            animation_content_from_frames(picker, decoder.into_frames(), size)
        }
        _ => None,
    }
}

pub(in crate::app) enum AnimationProbeOutcome {
    StaticFallback,
    Handled,
}

pub(in crate::app) struct AnimationRequestContext<'a> {
    pub(in crate::app) picker: &'a Picker,
    pub(in crate::app) load_control: &'a LoadControl,
    pub(in crate::app) done_tx: &'a Sender<LoadResult>,
    pub(in crate::app) req: &'a LoadRequest,
    pub(in crate::app) dims: (u32, u32),
    pub(in crate::app) size: Size,
    pub(in crate::app) kind: OriginalLoadKind,
}

pub(in crate::app) fn try_handle_animation_original(
    ctx: &AnimationRequestContext<'_>,
    path: &Path,
) -> AnimationProbeOutcome {
    let Some(format) = image::ImageFormat::from_path(path).ok() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Ok(file) = File::open(path) else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let reader = BufReader::new(file);

    match format {
        image::ImageFormat::Gif => {
            let Ok(decoder) = image::codecs::gif::GifDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        image::ImageFormat::Png => {
            let Ok(decoder) = image::codecs::png::PngDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            let Ok(decoder) = decoder.apng() else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        image::ImageFormat::WebP => {
            let Ok(decoder) = image::codecs::webp::WebPDecoder::new(reader) else {
                return AnimationProbeOutcome::StaticFallback;
            };
            handle_animation_frames(ctx, decoder.into_frames())
        }
        _ => AnimationProbeOutcome::StaticFallback,
    }
}

fn handle_animation_frames<I>(ctx: &AnimationRequestContext<'_>, frames: I) -> AnimationProbeOutcome
where
    I: IntoIterator<Item = image::ImageResult<image::Frame>>,
{
    let mut frames = frames.into_iter();
    let Some(first) = frames.next() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Some(first) = first
        .ok()
        .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
    else {
        return AnimationProbeOutcome::StaticFallback;
    };
    if !ctx.load_control.allows(ctx.req) {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    let Some(second) = frames.next() else {
        return AnimationProbeOutcome::StaticFallback;
    };
    let Some(second) = second
        .ok()
        .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
    else {
        return AnimationProbeOutcome::StaticFallback;
    };
    if !ctx.load_control.allows(ctx.req) {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    if ctx.kind == OriginalLoadKind::Prefetch {
        let _ = ctx.done_tx.send(skipped_load_result(ctx.req.clone()));
        return AnimationProbeOutcome::Handled;
    }

    if !send_animation_started(ctx.done_tx, ctx.req, ctx.dims) {
        return AnimationProbeOutcome::Handled;
    }
    if !send_animation_frame(ctx.done_tx, ctx.req, 0, first) {
        return AnimationProbeOutcome::Handled;
    }
    if !ctx.load_control.allows(ctx.req) {
        let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
        return AnimationProbeOutcome::Handled;
    }
    if !send_animation_frame(ctx.done_tx, ctx.req, 1, second) {
        return AnimationProbeOutcome::Handled;
    }

    for (frame_count, frame) in (2usize..).zip(frames) {
        if !ctx.load_control.allows(ctx.req) {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        }
        if frame_count == MAX_ANIMATION_FRAMES {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        }
        let Some(frame) = frame
            .ok()
            .and_then(|frame| animation_frame_from_image_frame(ctx.picker, frame, ctx.size))
        else {
            let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
            return AnimationProbeOutcome::Handled;
        };
        if !send_animation_frame(ctx.done_tx, ctx.req, frame_count, frame) {
            return AnimationProbeOutcome::Handled;
        }
    }

    if !ctx.load_control.allows(ctx.req) {
        let _ = send_animation_finished(ctx.done_tx, ctx.req, false);
        return AnimationProbeOutcome::Handled;
    }
    let _ = send_animation_finished(ctx.done_tx, ctx.req, true);
    AnimationProbeOutcome::Handled
}

fn send_animation_started(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    dims: (u32, u32),
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationStarted { dims },
            dims: None,
        })
        .is_ok()
}

fn send_animation_frame(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    index: usize,
    frame: AnimationFrame,
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationFrame { index, frame },
            dims: None,
        })
        .is_ok()
}

fn send_animation_finished(
    done_tx: &Sender<LoadResult>,
    req: &LoadRequest,
    complete: bool,
) -> bool {
    done_tx
        .send(LoadResult {
            key: req.key.clone(),
            size: req.size.clone(),
            generation: req.generation,
            content: LoadContent::AnimationFinished { complete },
            dims: None,
        })
        .is_ok()
}

#[cfg(test)]
pub(in crate::app) fn animation_frames_estimated_bytes(
    frames: &[AnimationFrame],
    font_size: FontSize,
) -> usize {
    frames
        .iter()
        .map(|frame| animation_frame_estimated_bytes(frame, font_size))
        .sum()
}

pub(in crate::app) fn animation_frame_estimated_bytes(
    frame: &AnimationFrame,
    font_size: FontSize,
) -> usize {
    let size = frame.protocol.size();
    usize::from(size.width.max(1))
        .saturating_mul(usize::from(size.height.max(1)))
        .saturating_mul(usize::from(font_size.width.max(1)))
        .saturating_mul(usize::from(font_size.height.max(1)))
        .saturating_mul(4)
}
