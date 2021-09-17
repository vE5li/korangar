use notify::{ RecommendedWatcher, Watcher, RecursiveMode, DebouncedEvent, watcher };
use std::sync::mpsc::{ channel, Receiver };
use std::time::Duration;

pub struct ReloadWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<DebouncedEvent>,
    directory: &'static str,
}

impl ReloadWatcher {

    pub fn new(directory: &'static str, debounce_timer: u64) -> Self {
        let (sender, receiver) = channel();
        let mut watcher = watcher(sender, Duration::from_millis(debounce_timer)).unwrap();

        watcher.watch(directory, RecursiveMode::Recursive).unwrap();

        return Self { watcher, receiver, directory };
    }

    pub fn poll_event(&mut self) -> Option<String> {
        if let Ok(event) = self.receiver.try_recv() {
            if let DebouncedEvent::Write(path) = &event {
                return Some(path.to_str().unwrap().replace(self.directory, ""));
            }
        }
        return None;
    }
}
