use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::RawVersion;
use crate::backend::download::sources::{minecraft, java, modrinth};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Condvar, LazyLock};
use std::thread;
use std::collections::HashMap;

pub static DOWNLOAD_QUEUE: LazyLock<NetworkQueue> = LazyLock::new(NetworkQueue::new);

// ---------------------------------------------------------------------------
// Download/Progress Messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum DownloadMsg {
    Progress(String, f32),
    DetailedProgress {
        task: String,
        current: usize,
        total: usize,
        item_name: String,
        overall_progress: f32,
    },
    Error(String),
    Finished,
}

// ---------------------------------------------------------------------------
// Network Task & Job Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum NetworkTask {
    MinecraftDownload {
        version: RawVersion,
        loader: ModLoader,
        loader_version: Option<String>,
        data_path: PathBuf,
    },
    JavaDownload {
        package_id: String,
        target_dir: PathBuf,
    },
    ModrinthDownload {
        project_id: String,
        version_id: Option<String>,
        game_version: String,
        loader: ModLoader,
        mods_dir: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkJobStatus {
    Pending,
    Running { active_task_name: String, progress: f32 },
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct NetworkJob {
    pub id: String,
    pub title: String,
    pub tasks: Vec<NetworkTask>,
    pub status: NetworkJobStatus,
    pub log: Vec<String>,
}

// ---------------------------------------------------------------------------
// Centralized Thread-Safe Network Queue
// ---------------------------------------------------------------------------

pub struct NetworkQueue {
    inner: Arc<Mutex<QueueInner>>,
    cv: Arc<Condvar>,
}

struct QueueInner {
    jobs: Vec<NetworkJob>,
    worker_spawned: bool,
    senders: HashMap<String, std::sync::mpsc::Sender<DownloadMsg>>,
}

impl NetworkQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(QueueInner {
                jobs: Vec::new(),
                worker_spawned: false,
                senders: HashMap::new(),
            })),
            cv: Arc::new(Condvar::new()),
        }
    }

    pub fn add_job(&self, job: NetworkJob, progress_sender: std::sync::mpsc::Sender<DownloadMsg>) {
        let mut inner = self.inner.lock().unwrap();
        let job_id = job.id.clone();
        inner.jobs.push(job);
        inner.senders.insert(job_id, progress_sender);

        if !inner.worker_spawned {
            inner.worker_spawned = true;
            let inner_clone = self.inner.clone();
            let cv_clone = self.cv.clone();
            thread::spawn(move || {
                Self::worker_loop(inner_clone, cv_clone);
            });
        } else {
            self.cv.notify_one();
        }
    }

    pub fn get_jobs(&self) -> Vec<NetworkJob> {
        let inner = self.inner.lock().unwrap();
        inner.jobs.clone()
    }

    pub fn remove_job(&self, id: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.jobs.retain(|j| j.id != id);
        inner.senders.remove(id);
    }

    pub fn clear_finished_jobs(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.jobs.retain(|j| matches!(j.status, NetworkJobStatus::Pending | NetworkJobStatus::Running { .. }));
    }

    fn worker_loop(
        inner: Arc<Mutex<QueueInner>>,
        cv: Arc<Condvar>,
    ) {
        loop {
            // Find next Pending job in the unified list
            let (job_id, job) = {
                let mut guard = inner.lock().unwrap();
                let mut found = None;
                for j in &mut guard.jobs {
                    if matches!(j.status, NetworkJobStatus::Pending) {
                        j.status = NetworkJobStatus::Running {
                            active_task_name: "Starting...".to_string(),
                            progress: 0.0,
                        };
                        j.log.push("Job started...".to_string());
                        found = Some((j.id.clone(), j.clone()));
                        break;
                    }
                }
                
                // Send an initial progress update to trigger the UI to refresh immediately
                if let Some((ref j_id, _)) = found {
                    if let Some(tx) = guard.senders.get(j_id) {
                        let _ = tx.send(DownloadMsg::Progress("Starting...".to_string(), 0.0));
                    }
                }

                match found {
                    Some(f) => f,
                    None => {
                        let _guard = cv.wait(guard).unwrap();
                        continue;
                    }
                }
            };

            let mut job_failed = false;
            let mut job_err = String::new();
            let total_tasks = job.tasks.len();
            let job_id_clone = job_id.clone();

            for (idx, task) in job.tasks.iter().enumerate() {
                if job_failed {
                    break;
                }

                let task_start_progress = idx as f32 / total_tasks as f32;
                let task_weight = 1.0 / total_tasks as f32;

                let inner_c = inner.clone();
                let job_id_c = job_id_clone.clone();

                let status_update = move |msg: String, progress: f32| {
                    let overall = task_start_progress + (progress * task_weight);
                    let mut guard = inner_c.lock().unwrap();
                    if let Some(j) = guard.jobs.iter_mut().find(|j| j.id == job_id_c) {
                        j.status = NetworkJobStatus::Running {
                            active_task_name: msg.clone(),
                            progress: overall,
                        };
                        j.log.push(format!("[{:.0}%] {}", overall * 100.0, msg));
                    }
                    if let Some(tx) = guard.senders.get(&job_id_c) {
                        let _ = tx.send(DownloadMsg::Progress(msg, overall));
                    }
                };

                let res = match task {
                    NetworkTask::MinecraftDownload {
                        version,
                        loader,
                        loader_version,
                        data_path,
                    } => minecraft::download_minecraft_data_internal(
                        version,
                        loader,
                        loader_version.as_deref(),
                        data_path,
                        status_update,
                    ),
                    NetworkTask::JavaDownload {
                        package_id,
                        target_dir,
                    } => {
                        let (java_tx, java_rx) = std::sync::mpsc::channel();
                        let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        
                        let pkg = package_id.clone();
                        let dir = target_dir.clone();
                        thread::spawn(move || {
                            java::download_and_extract_with_progress(&pkg, &dir, cancel_flag, move |prog| {
                                let _ = java_tx.send(prog);
                            });
                        });

                        let mut task_res = Ok(());
                        while let Ok(prog) = java_rx.recv() {
                            match prog {
                                java::JavaDownloadProgress::Downloading { current, total } => {
                                    let prog_pct = if total > 0 { current as f32 / total as f32 } else { 0.0 };
                                    status_update(format!("Downloading Java... ({:.1}%)", prog_pct * 100.0), prog_pct);
                                }
                                java::JavaDownloadProgress::Extracting => {
                                    status_update("Extracting Java runtime...".to_string(), 0.9);
                                }
                                java::JavaDownloadProgress::Finished(_) => {
                                    status_update("Java installation complete".to_string(), 1.0);
                                    break;
                                }
                                java::JavaDownloadProgress::Error(e) => {
                                    task_res = Err(e);
                                    break;
                                }
                            }
                        }
                        task_res
                    }
                    NetworkTask::ModrinthDownload {
                        project_id,
                        version_id,
                        game_version,
                        loader,
                        mods_dir,
                    } => {
                        let callback = status_update.clone();
                        modrinth::install_mod_with_dependencies(
                            project_id,
                            version_id.clone(),
                            game_version,
                            loader.clone(),
                            mods_dir,
                            move |msg, progress| {
                                callback(msg, progress);
                            },
                        ).map(|_| ())
                    }
                };

                if let Err(e) = res {
                    job_failed = true;
                    job_err = e;
                }
            }

            let final_status = if job_failed {
                NetworkJobStatus::Failed(job_err.clone())
            } else {
                NetworkJobStatus::Completed
            };

            // 1. Update status in DOWNLOAD_QUEUE jobs list FIRST
            {
                let mut guard = inner.lock().unwrap();
                if let Some(j) = guard.jobs.iter_mut().find(|j| j.id == job_id_clone) {
                    j.status = final_status;
                    if job_failed {
                        j.log.push(format!("Job failed: {}", job_err));
                    } else {
                        j.log.push("Job completed successfully.".to_string());
                    }
                }
            }

            // 2. Send Finished or Error message SECOND (to trigger UI Refresh)
            {
                let guard = inner.lock().unwrap();
                if let Some(tx) = guard.senders.get(&job_id_clone) {
                    if job_failed {
                        let _ = tx.send(DownloadMsg::Error(job_err.clone()));
                    } else {
                        let _ = tx.send(DownloadMsg::Finished);
                    }
                }
            }

            // 3. Remove the sender LAST
            {
                let mut guard = inner.lock().unwrap();
                guard.senders.remove(&job_id_clone);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// High-Level Unified API Delegation (Shields UI from raw API/caching logic)
// ---------------------------------------------------------------------------

pub fn download_minecraft_data(
    version: &RawVersion,
    loader: &ModLoader,
    loader_version: Option<&str>,
    data_path: &Path,
    sender: &std::sync::mpsc::Sender<DownloadMsg>,
) -> Result<(), String> {
    let sender_clone = sender.clone();
    minecraft::download_minecraft_data_internal(
        version,
        loader,
        loader_version,
        data_path,
        move |msg, progress| {
            let _ = sender_clone.send(DownloadMsg::Progress(msg, progress));
        },
    )
}

pub use java::{
    get_available_packages as fetch_java_packages,
    JavaPackage, JavaDownloadProgress,
};

pub use modrinth::{
    clear_caches as clear_modrinth_caches,
    get_project as fetch_modrinth_project,
    get_project_versions as fetch_modrinth_versions,
    search_mods as search_modrinth_mods,
    ModProject, ModSearchResult, ModVersion, ModDependency, ModFile,
};
