use notify::Watcher as _;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;

#[macro_use]
extern crate log;

pub struct FileWatcher {
    filename: OsString,
    rx: mpsc::Receiver<notify::RawEvent>,
    _watcher: notify::RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(path: &Path) -> Result<FileWatcher, Box<dyn Error>> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::raw_watcher(tx)?;
        let parent_dir = path.parent().unwrap_or(Path::new(""));
        let parent_dir = if !parent_dir.as_os_str().is_empty() { parent_dir } else { Path::new(".") };
        let filename = OsString::from(path.file_name().expect("path must have filename"));
        info!("watching parent directory {:?}", parent_dir);
        watcher.watch(parent_dir, notify::RecursiveMode::NonRecursive)?;

        Ok(FileWatcher {
            filename,
            rx,
            _watcher: watcher,
        })
    }
    pub fn wait_for_event(&self) {
        loop {
            match self.rx.recv().unwrap() {
                notify::RawEvent { path: Some(p), op: Ok(op), .. } if p.file_name() == Some(&self.filename) && op.intersects(notify::Op::CREATE | notify::Op::RENAME | notify::Op::REMOVE) => break,
                notify::RawEvent { path: Some(_), op: Ok(_), .. } => continue,
                weird => {
                    warn!("weird event: {:?}", weird);
                    continue;
                },
            }
        }
    }
}

pub struct RemoveFileOnDrop(pub Arc<Mutex<bool>>, pub String);

impl Drop for RemoveFileOnDrop {
    fn drop(&mut self) {
        remove_file_on_quit(&self.0, &self.1);
    }
}

pub fn remove_file_on_quit(write_mutex: &Mutex<bool>, file: &str) {
    info!("removing file before quitting");
    {
        let mut _guard = write_mutex.lock().unwrap_or_else(|e| e.into_inner());
        // Already removed?
        if *_guard {
            return;
        }
        if let Err(e) = remove_file_if_present(file) {
            error!("failed to remove file: {}", e);
        }
        *_guard = true;
    }
}

pub fn remove_file_if_present(file: &str) -> io::Result<()> {
    let result = fs::remove_file(file);
    if let Err(e) = &result {
        if e.kind() == io::ErrorKind::NotFound {
            return Ok(());
        }
    }
    result
}
