use super::animation::{
    make_protocol, static_rgba_content, try_handle_animation_original, AnimationProbeOutcome,
    AnimationRequestContext,
};
use super::*;

pub(in crate::app) struct OriginalRequestParts<'a> {
    pub(in crate::app) path: &'a Path,
    pub(in crate::app) key: ImageCacheKey,
    pub(in crate::app) generation: u64,
    pub(in crate::app) w: u16,
    pub(in crate::app) h: u16,
    pub(in crate::app) kind: OriginalLoadKind,
}

pub(in crate::app) fn load_content_is_terminal(content: &LoadContent) -> bool {
    matches!(
        content,
        LoadContent::Thumbnail(_)
            | LoadContent::Original(_)
            | LoadContent::AnimationFinished { .. }
            | LoadContent::Skipped
    )
}

#[cfg(test)]
pub(in crate::app) fn process_load_request_with_control(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
) -> Option<LoadResult> {
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();
    process_load_request_with_control_to_sender(picker, load_control, req, &done_tx);
    done_rx.try_recv().ok()
}

pub(in crate::app) fn process_load_request_with_control_to_sender(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
    done_tx: &Sender<LoadResult>,
) {
    if !load_control.allows(&req) {
        let _ = done_tx.send(skipped_load_result(req));
        return;
    }

    if let Some(result) = process_load_request(picker, load_control, req, done_tx) {
        let _ = done_tx.send(result);
    }
}

pub(in crate::app) fn skipped_load_result(req: LoadRequest) -> LoadResult {
    let LoadRequest {
        key,
        size,
        generation,
        ..
    } = req;

    LoadResult {
        key,
        size,
        generation,
        content: LoadContent::Skipped,
        dims: None,
    }
}

fn process_load_request(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
    done_tx: &Sender<LoadResult>,
) -> Option<LoadResult> {
    let LoadRequest {
        key,
        path,
        size,
        generation,
        ..
    } = req;
    match size {
        LoadSize::Thumbnail { w, h } => process_thumbnail_request_with_control(
            picker,
            load_control,
            LoadRequest {
                key,
                path,
                size: LoadSize::Thumbnail { w, h },
                generation,
            },
            w,
            h,
        ),
        LoadSize::Original { w, h, kind } => process_original_request(
            picker,
            load_control,
            done_tx,
            OriginalRequestParts {
                path: path.as_path(),
                key,
                generation,
                w,
                h,
                kind,
            },
        ),
    }
}

fn process_thumbnail_request_with_control(
    picker: &Picker,
    load_control: &LoadControl,
    req: LoadRequest,
    w: u16,
    h: u16,
) -> Option<LoadResult> {
    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }

    let img = image::open(&req.path).ok()?;
    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }

    let font_size = picker.font_size();
    let pixel_w = w as u32 * font_size.width as u32 * 2;
    let pixel_h = h as u32 * font_size.height as u32 * 2;
    let dims = Some((img.width(), img.height()));
    let thumb = img.thumbnail(pixel_w, pixel_h);
    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }

    thumbnail_result_from_image(picker, req, w, h, dims, thumb)
}

#[cfg(any(test, feature = "bench-internals"))]
pub(in crate::app) fn process_thumbnail_request(
    picker: &Picker,
    path: &Path,
    key: ImageCacheKey,
    generation: u64,
    w: u16,
    h: u16,
) -> Option<LoadResult> {
    let img = image::open(path).ok()?;
    let req = LoadRequest {
        key,
        path: path.to_path_buf(),
        size: LoadSize::Thumbnail { w, h },
        generation,
    };
    let font_size = picker.font_size();
    let pixel_w = w as u32 * font_size.width as u32 * 2;
    let pixel_h = h as u32 * font_size.height as u32 * 2;
    let dims = Some((img.width(), img.height()));
    let thumb = img.thumbnail(pixel_w, pixel_h);
    thumbnail_result_from_image(picker, req, w, h, dims, thumb)
}

fn thumbnail_result_from_image(
    picker: &Picker,
    req: LoadRequest,
    w: u16,
    h: u16,
    dims: Option<(u32, u32)>,
    thumb: image::DynamicImage,
) -> Option<LoadResult> {
    let protocol = make_protocol(picker, thumb, Size::new(w, h), ProtocolFilterType::Nearest)?;
    let LoadRequest {
        key,
        size,
        generation,
        ..
    } = req;

    Some(LoadResult {
        key,
        size,
        generation,
        content: LoadContent::Thumbnail(protocol),
        dims,
    })
}

pub(in crate::app) fn process_original_request(
    picker: &Picker,
    load_control: &LoadControl,
    done_tx: &Sender<LoadResult>,
    parts: OriginalRequestParts<'_>,
) -> Option<LoadResult> {
    let OriginalRequestParts {
        path,
        key,
        generation,
        w,
        h,
        kind,
    } = parts;
    let size = LoadSize::Original { w, h, kind };
    let req = LoadRequest {
        key: key.clone(),
        path: path.to_path_buf(),
        size: size.clone(),
        generation,
    };
    let dims = image::image_dimensions(path).ok()?;
    let protocol_size = Size::new(w.max(1), h.max(1));

    if should_probe_animation(path) {
        let animation_ctx = AnimationRequestContext {
            picker,
            load_control,
            done_tx,
            dims,
            size: protocol_size,
            kind,
            req: &req,
        };
        match try_handle_animation_original(&animation_ctx, path) {
            AnimationProbeOutcome::Handled => return None,
            AnimationProbeOutcome::StaticFallback => {}
        }
    }

    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }
    let content = static_rgba_content(image::open(path).ok()?.into_rgba8());
    if !load_control.allows(&req) {
        return Some(skipped_load_result(req));
    }

    Some(LoadResult {
        key,
        size,
        generation,
        content: LoadContent::Original(content),
        dims: Some(dims),
    })
}

fn should_probe_animation(path: &Path) -> bool {
    matches!(
        image::ImageFormat::from_path(path).ok(),
        Some(image::ImageFormat::Gif | image::ImageFormat::Png | image::ImageFormat::WebP)
    )
}
