use std::path::Path;

use notify::{Event, RecommendedWatcher, Watcher};
use tasks::{
    sync_channel::{unbounded, Receiver},
    sync_lock::Mutex,
};

pub(crate) struct FileSpy {
    watcher: Mutex<RecommendedWatcher>,
    rx: Receiver<notify::Result<Event>>,
}

impl FileSpy {
    pub(crate) fn new() -> Self {
        let (tx, rx) = unbounded();
        let watcher = notify::immediate_watcher(move |e| {
            tx.send(e)
                .expect("[FileSpy] (notify callback) failed to send event");
        })
        .expect("[FileSpy] (new) failed to create watcher");
        Self {
            watcher: Mutex::new(watcher),
            rx,
        }
    }

    /// Get a reference to the file spy's rx.
    pub(crate) fn rx(&self) -> &Receiver<notify::Result<Event>> {
        &self.rx
    }

    pub(crate) fn watch_asset<P: AsRef<Path>>(&self, path: P) {
        log::debug!("Will watch: {:?}", path.as_ref());
        self.watcher
            .lock()
            .watch(path, notify::RecursiveMode::NonRecursive)
            .expect("[FileSpy] (watch_asset) failed to watch path");
    }
}

impl Default for FileSpy {
    fn default() -> Self {
        Self::new()
    }
}
