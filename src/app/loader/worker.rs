use super::*;

pub fn spawn_image_loader(
    picker: Picker,
    _paths: Vec<std::path::PathBuf>,
    load_control: LoadControl,
) -> (Sender<LoadRequest>, Receiver<LoadResult>) {
    let (load_tx, load_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<LoadResult>();

    let (thumb_tx, thumb_rx) = std::sync::mpsc::channel::<LoadRequest>();
    let (original_tx, original_rx) = std::sync::mpsc::channel::<LoadRequest>();

    std::thread::spawn(move || {
        while let Ok(req) = load_rx.recv() {
            let routed = match &req.size {
                LoadSize::Thumbnail { .. } => thumb_tx.send(req),
                LoadSize::Original { .. } => original_tx.send(req),
            };
            if routed.is_err() {
                break;
            }
        }
    });

    spawn_loader_workers(
        picker.clone(),
        done_tx.clone(),
        thumb_rx,
        load_control.clone(),
        3,
    );
    spawn_loader_workers(picker, done_tx, original_rx, load_control, 1);

    (load_tx, done_rx)
}

fn spawn_loader_workers(
    picker: Picker,
    done_tx: Sender<LoadResult>,
    load_rx: Receiver<LoadRequest>,
    load_control: LoadControl,
    workers: usize,
) {
    let rx = Arc::new(std::sync::Mutex::new(load_rx));
    for _ in 0..workers {
        let picker = picker.clone();
        let done_tx = done_tx.clone();
        let rx = Arc::clone(&rx);
        let load_control = load_control.clone();

        std::thread::spawn(move || loop {
            let req = {
                let rx = rx.lock().unwrap();
                match rx.recv() {
                    Ok(req) => req,
                    Err(_) => return,
                }
            };

            process_load_request_with_control_to_sender(&picker, &load_control, req, &done_tx);
        });
    }
}
