//! Autosave with a debounce: a background thread writes a sheet once no new text has arrived for the
//! delay, so typing does not write on every key but a pause (or quitting) always saves.

use std::collections::HashMap;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::{SheetBook, SheetId};

enum Msg {
    Submit(SheetId, Vec<String>),
    Flush(Sender<()>),
    Stop,
}

pub struct Autosaver {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Autosaver {
    /// Starts the writer for `book`; text submitted is written `delay` after the last submit for that sheet.
    pub fn start(book: SheetBook, delay: Duration) -> Autosaver {
        let (tx, rx) = mpsc::channel::<Msg>();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let errs = errors.clone();
        let thread = std::thread::spawn(move || {
            let mut pending: HashMap<SheetId, (Instant, Vec<String>)> = HashMap::new();
            let write = |pending: &mut HashMap<SheetId, (Instant, Vec<String>)>, only_due: bool| {
                let now = Instant::now();
                let due: Vec<SheetId> = pending
                    .iter()
                    .filter(|(_, (at, _))| !only_due || now >= *at + delay)
                    .map(|(id, _)| id.clone())
                    .collect();
                for id in due {
                    if let Some((_, lines)) = pending.remove(&id)
                        && let Err(e) = book.save(&id, &lines)
                        && let Ok(mut v) = errs.lock()
                    {
                        v.push(e.to_string());
                    }
                }
            };
            loop {
                let wait = pending
                    .values()
                    .map(|(at, _)| (*at + delay).saturating_duration_since(Instant::now()))
                    .min();
                let msg = match wait {
                    Some(w) => rx.recv_timeout(w),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                match msg {
                    Ok(Msg::Submit(id, lines)) => {
                        pending.insert(id, (Instant::now(), lines));
                    }
                    Ok(Msg::Flush(done)) => {
                        write(&mut pending, false);
                        let _ = done.send(());
                    }
                    Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => {
                        write(&mut pending, false);
                        return;
                    }
                    Err(RecvTimeoutError::Timeout) => write(&mut pending, true),
                }
            }
        });
        Autosaver {
            tx,
            thread: Some(thread),
            errors,
        }
    }

    /// Queues `lines` as the sheet's text, replacing any earlier text still waiting.
    pub fn submit(&self, id: &SheetId, lines: Vec<String>) {
        let _ = self.tx.send(Msg::Submit(id.clone(), lines));
    }

    /// Writes everything waiting now and returns when it is on disk.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Msg::Flush(done)).is_ok() {
            let _ = wait.recv();
        }
    }

    /// Write errors since the last call, oldest first.
    pub fn take_errors(&self) -> Vec<String> {
        self.errors
            .lock()
            .map(|mut v| std::mem::take(&mut *v))
            .unwrap_or_default()
    }
}

impl Drop for Autosaver {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, SheetBook, SheetId) {
        let d = tempfile::tempdir().unwrap();
        let b = SheetBook::open(d.path().join("s")).unwrap();
        let id = b.create(&["old"]).unwrap();
        (d, SheetBook::open(b.dir()).unwrap(), id)
    }

    fn lines(b: &SheetBook, id: &SheetId) -> Vec<String> {
        b.read(id).unwrap()
    }

    #[test]
    fn nothing_is_written_before_the_delay() {
        let (_d, b, id) = setup();
        let reader = SheetBook::open(b.dir()).unwrap();
        let a = Autosaver::start(b, Duration::from_millis(600));
        a.submit(&id, vec!["new".into()]);
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(lines(&reader, &id), vec!["old"]);
    }

    #[test]
    fn the_text_is_written_after_the_delay() {
        let (_d, b, id) = setup();
        let reader = SheetBook::open(b.dir()).unwrap();
        let a = Autosaver::start(b, Duration::from_millis(50));
        a.submit(&id, vec!["new".into()]);
        let start = Instant::now();
        while lines(&reader, &id) != vec!["new"] {
            assert!(start.elapsed() < Duration::from_secs(5), "never written");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn a_steady_stream_of_edits_keeps_postponing_the_write() {
        let (_d, b, id) = setup();
        let reader = SheetBook::open(b.dir()).unwrap();
        let a = Autosaver::start(b, Duration::from_millis(300));
        for i in 0..6 {
            a.submit(&id, vec![format!("v{i}")]);
            std::thread::sleep(Duration::from_millis(100));
            assert_eq!(
                lines(&reader, &id),
                vec!["old"],
                "still waiting at edit {i}"
            );
        }
        a.flush();
        assert_eq!(
            lines(&reader, &id),
            vec!["v5"],
            "only the last text is written"
        );
    }

    #[test]
    fn flush_writes_at_once_and_drop_writes_what_is_left() {
        let (_d, b, id) = setup();
        let reader = SheetBook::open(b.dir()).unwrap();
        let a = Autosaver::start(b, Duration::from_secs(60));
        a.submit(&id, vec!["one".into()]);
        a.flush();
        assert_eq!(lines(&reader, &id), vec!["one"]);
        a.submit(&id, vec!["two".into()]);
        drop(a);
        assert_eq!(lines(&reader, &id), vec!["two"]);
    }

    #[test]
    fn several_sheets_wait_and_write_independently() {
        let (_d, b, a_id) = setup();
        let b_id = b.create(&["other"]).unwrap();
        let reader = SheetBook::open(b.dir()).unwrap();
        let s = Autosaver::start(b, Duration::from_secs(60));
        s.submit(&a_id, vec!["A".into()]);
        s.submit(&b_id, vec!["B".into()]);
        s.flush();
        assert_eq!(
            (lines(&reader, &a_id), lines(&reader, &b_id)),
            (vec!["A".to_owned()], vec!["B".to_owned()])
        );
    }

    #[test]
    fn a_failed_write_is_reported_not_lost_silently() {
        let (d, b, id) = setup();
        let a = Autosaver::start(b, Duration::from_millis(10));
        std::fs::remove_dir_all(d.path().join("s")).unwrap();
        a.submit(&id, vec!["x".into()]);
        a.flush();
        assert_eq!(a.take_errors().len(), 1);
        assert!(a.take_errors().is_empty());
    }
}
