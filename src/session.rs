use anyhow::{Context, Result};
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex, mpsc},
    thread,
    time::Duration,
};
use windows::{
    Win32::Storage::FileSystem::{MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW},
    core::PCWSTR,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct TabMeta {
    pub id: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub cursor: usize,
    pub anchor: usize,
    pub scroll: usize,
    pub syntax: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub active: usize,
    pub tabs: Vec<TabMeta>,
    #[serde(default)]
    pub appearance: crate::appearance::Appearance,
}
#[derive(Clone)]
pub struct Snapshot {
    pub manifest: Manifest,
    pub texts: Vec<(u64, u64, Rope)>,
}

pub fn directory() -> PathBuf {
    if let Some(path) = std::env::var_os("WINY_DATA_DIR") {
        return PathBuf::from(path);
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("winY")
        .join("temp")
}
pub fn atomic_write(path: &Path, write: impl FnOnce(&mut fs::File) -> Result<()>) -> Result<()> {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temporary = path.with_extension(format!("winy-{}-{suffix}.pending", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("Writing {}", temporary.display()))?;
    write(&mut file)?;
    file.sync_all()?;
    drop(file);
    let wide = |p: &Path| {
        p.as_os_str()
            .to_string_lossy()
            .encode_utf16()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    unsafe {
        MoveFileExW(
            PCWSTR(wide(&temporary).as_ptr()),
            PCWSTR(wide(path).as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )?;
    }
    Ok(())
}
pub fn load() -> Result<(Manifest, Vec<String>)> {
    let directory = directory();
    fs::create_dir_all(&directory)?;
    let path = directory.join("session.json");
    if !path.exists() {
        return Ok((Manifest::default(), vec![]));
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(path)?)?;
    let mut texts = Vec::new();
    for tab in &manifest.tabs {
        texts.push(
            fs::read_to_string(directory.join(format!("{}.txt", tab.id)))
                .with_context(|| format!("Restauration de l’onglet {}", tab.title))?,
        );
    }
    Ok((manifest, texts))
}

struct Pending {
    snapshot: Option<Snapshot>,
    stop: bool,
}
pub struct Writer {
    state: Arc<(Mutex<Pending>, Condvar)>,
    thread: Option<thread::JoinHandle<()>>,
    pub errors: mpsc::Receiver<String>,
}
impl Writer {
    pub fn new(directory: PathBuf) -> Self {
        let state = Arc::new((
            Mutex::new(Pending {
                snapshot: None,
                stop: false,
            }),
            Condvar::new(),
        ));
        let worker_state = state.clone();
        let (errors_tx, errors) = mpsc::channel();
        let thread = thread::spawn(move || {
            let mut versions = std::collections::HashMap::new();
            loop {
                let (lock, wake) = &*worker_state;
                let mut pending = lock.lock().expect("session mutex");
                while pending.snapshot.is_none() && !pending.stop {
                    pending = wake.wait(pending).expect("session wake");
                }
                let deadline = std::time::Instant::now() + Duration::from_millis(250);
                while !pending.stop && std::time::Instant::now() < deadline {
                    pending = wake
                        .wait_timeout(
                            pending,
                            deadline.saturating_duration_since(std::time::Instant::now()),
                        )
                        .expect("save debounce")
                        .0;
                }
                let snapshot = pending.snapshot.take();
                let stop = pending.stop;
                drop(pending);
                if let Some(snapshot) = snapshot {
                    let result = (|| -> Result<()> {
                        fs::create_dir_all(&directory)?;
                        for (id, revision, text) in snapshot.texts {
                            if versions.get(&id) == Some(&revision) {
                                continue;
                            }
                            atomic_write(&directory.join(format!("{id}.txt")), |file| {
                                for chunk in text.chunks() {
                                    file.write_all(chunk.as_bytes())?;
                                }
                                Ok(())
                            })?;
                            versions.insert(id, revision);
                        }
                        atomic_write(&directory.join("session.json"), |file| {
                            serde_json::to_writer(file, &snapshot.manifest)?;
                            Ok(())
                        })
                    })();
                    if let Err(error) = result
                        && errors_tx
                            .send(format!("Could not save session: {error:#}"))
                            .is_err()
                    {
                        break;
                    }
                }
                if stop {
                    break;
                }
            }
        });
        Self {
            state,
            thread: Some(thread),
            errors,
        }
    }
    pub fn submit(&self, snapshot: Snapshot) {
        let mut state = self.state.0.lock().expect("session mutex");
        state.snapshot = Some(snapshot);
        self.state.1.notify_one();
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        {
            self.state.0.lock().expect("session mutex").stop = true;
        }
        self.state.1.notify_one();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("Session writer panicked");
        }
        for error in self.errors.try_iter() {
            crate::native::message(&error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flush_on_shutdown_and_atomic_replacement() {
        let dir = std::env::temp_dir().join(format!("winy-test-{}", std::process::id()));
        let writer = Writer::new(dir.clone());
        writer.submit(Snapshot {
            manifest: Manifest::default(),
            texts: vec![(1, 0, Rope::from_str("first"))],
        });
        writer.submit(Snapshot {
            manifest: Manifest::default(),
            texts: vec![(1, 1, Rope::from_str("é🙂 final"))],
        });
        drop(writer);
        assert_eq!(fs::read_to_string(dir.join("1.txt")).unwrap(), "é🙂 final");
        assert!(dir.join("session.json").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
