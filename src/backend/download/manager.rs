use crate::backend::download::sources::{java, minecraft, modrinth};
use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::RawVersion;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::thread;

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

#[derive(Debug, Clone, PartialEq)]
pub enum TaskItemStatus {
    Success,
    Failed(String),
    Pending,
    Running(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadedItemType {
    Mod,
    ResourcePack,
    ShaderPack,
    World,
    MinecraftComponent,
    Java,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskItemDetail {
    pub name: String,
    pub status: TaskItemStatus,
    pub item_type: DownloadedItemType,
}

pub struct TaskContext<'a> {
    pub progress: &'a (dyn Fn(String, f32) + Send + Sync),
    pub item: &'a (dyn Fn(String, TaskItemStatus, DownloadedItemType) + Send + Sync),
}

pub trait DownloadTask: Send + Sync + std::fmt::Debug {
    fn run(&self, ctx: &TaskContext) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkJobStatus {
    Pending,
    Running {
        active_task_name: String,
        progress: f32,
    },
    Completed,
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct NetworkJob {
    pub id: String,
    pub title: String,
    pub tasks: Vec<Arc<dyn DownloadTask>>,
    pub status: NetworkJobStatus,
    pub log: Vec<String>,
    pub items: Vec<TaskItemDetail>,
}

// ---------------------------------------------------------------------------
// Concrete Download Tasks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MinecraftDownloadTask {
    pub version: RawVersion,
    pub loader: ModLoader,
    pub loader_version: Option<String>,
    pub data_path: PathBuf,
}

impl DownloadTask for MinecraftDownloadTask {
    fn run(&self, ctx: &TaskContext) -> Result<(), String> {
        minecraft::download_minecraft_data_internal(
            &self.version,
            &self.loader,
            self.loader_version.as_deref(),
            &self.data_path,
            ctx.progress,
            ctx.item,
        )
    }
}

#[derive(Debug, Clone)]
pub struct JavaDownloadTask {
    pub package_id: String,
    pub target_dir: PathBuf,
}

impl DownloadTask for JavaDownloadTask {
    fn run(&self, ctx: &TaskContext) -> Result<(), String> {
        let (java_tx, java_rx) = std::sync::mpsc::channel();
        let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let pkg = self.package_id.clone();
        let dir = self.target_dir.clone();
        thread::spawn(move || {
            java::download_and_extract_with_progress(
                &pkg,
                &dir,
                cancel_flag,
                move |prog| {
                    let _ = java_tx.send(prog);
                },
            );
        });

        (ctx.item)(
            format!("Java Runtime ({})", self.package_id),
            TaskItemStatus::Running("Starting...".to_string()),
            DownloadedItemType::Java,
        );

        let mut task_res = Ok(());
        while let Ok(prog) = java_rx.recv() {
            match prog {
                java::JavaDownloadProgress::Downloading { current, total } => {
                    let prog_pct = if total > 0 {
                        current as f32 / total as f32
                    } else {
                        0.0
                    };
                    let detail_str = format!("Downloading... ({:.1}%)", prog_pct * 100.0);
                    (ctx.progress)(
                        "Downloading Java...".to_string(),
                        prog_pct,
                    );
                    (ctx.item)(
                        format!("Java Runtime ({})", self.package_id),
                        TaskItemStatus::Running(detail_str),
                        DownloadedItemType::Java,
                    );
                }
                java::JavaDownloadProgress::Extracting => {
                    (ctx.progress)("Extracting Java runtime...".to_string(), 0.9);
                    (ctx.item)(
                        format!("Java Runtime ({})", self.package_id),
                        TaskItemStatus::Running("Extracting runtime...".to_string()),
                        DownloadedItemType::Java,
                    );
                }
                java::JavaDownloadProgress::Finished(_) => {
                    (ctx.progress)("Java installation complete".to_string(), 1.0);
                    (ctx.item)(
                        format!("Java Runtime ({})", self.package_id),
                        TaskItemStatus::Success,
                        DownloadedItemType::Java,
                    );
                    break;
                }
                java::JavaDownloadProgress::Error(e) => {
                    (ctx.item)(
                        format!("Java Runtime ({})", self.package_id),
                        TaskItemStatus::Failed(e.clone()),
                        DownloadedItemType::Java,
                    );
                    task_res = Err(e);
                    break;
                }
            }
        }
        task_res
    }
}

#[derive(Debug, Clone)]
pub struct ModrinthDownloadTask {
    pub project_id: String,
    pub version_id: Option<String>,
    pub game_version: String,
    pub loader: ModLoader,
    pub mods_dir: PathBuf,
    pub old_filename: Option<String>,
}

impl DownloadTask for ModrinthDownloadTask {
    fn run(&self, ctx: &TaskContext) -> Result<(), String> {
        let res = modrinth::install_mod_with_dependencies(
            &self.project_id,
            self.version_id.clone(),
            &self.game_version,
            self.loader.clone(),
            &self.mods_dir,
            ctx.progress,
            ctx.item,
        );

        if let Ok(ref downloaded_paths) = res {
            if let Some(old_fn) = &self.old_filename {
                let is_temp = self.mods_dir.file_name().and_then(|n| n.to_str()) != Some("mods");
                if is_temp {
                    let target_dir = self.mods_dir.parent().unwrap().join("mods");
                    if !target_dir.exists() {
                        let _ = std::fs::create_dir_all(&target_dir);
                    }
                    for path in downloaded_paths {
                        if let Some(fname) = path.file_name() {
                            let dest = target_dir.join(fname);
                            let dest_is_old = dest.file_name() == Some(std::ffi::OsStr::new(old_fn));
                            let _ = std::fs::rename(path, &dest);
                            if !dest_is_old {
                                let old_path = target_dir.join(old_fn);
                                let _ = std::fs::remove_file(old_path);
                            }
                        }
                    }
                    let marker_path = self.mods_dir.join(format!("{}.success", old_fn));
                    let _ = std::fs::File::create(marker_path);
                } else {
                    let old_path = self.mods_dir.join(old_fn);
                    let _ = std::fs::remove_file(old_path);
                }
            }
        }

        res.map(|_| ())
    }
}

#[derive(Debug, Clone)]
pub struct ModrinthModpackDownloadTask {
    pub name: String,
    pub download_url: String,
    pub instances_path: PathBuf,
}

impl DownloadTask for ModrinthModpackDownloadTask {
    fn run(&self, ctx: &TaskContext) -> Result<(), String> {
        crate::backend::instance::modpack::install_mrpack(
            &self.name,
            &self.download_url,
            &self.instances_path,
            ctx.progress,
            ctx.item,
        )
        .map(|_| ())
    }
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

impl Default for NetworkQueue {
    fn default() -> Self {
        Self::new()
    }
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
        inner.jobs.retain(|j| {
            matches!(
                j.status,
                NetworkJobStatus::Pending | NetworkJobStatus::Running { .. }
            )
        });
    }

    pub fn retry_job(&self, id: &str, progress_sender: std::sync::mpsc::Sender<DownloadMsg>) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(j) = inner.jobs.iter_mut().find(|j| j.id == id) {
            if let NetworkJobStatus::Failed(_) = j.status {
                j.status = NetworkJobStatus::Pending;
                j.log.push("Retrying failed items...".to_string());
                for item in &mut j.items {
                    if let TaskItemStatus::Failed(_) = item.status {
                        item.status = TaskItemStatus::Pending;
                    }
                }
                inner.senders.insert(id.to_string(), progress_sender);
                self.cv.notify_one();
            }
        }
    }

    fn worker_loop(inner: Arc<Mutex<QueueInner>>, cv: Arc<Condvar>) {
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

            let mut failed_task_errors = Vec::new();
            let total_tasks = job.tasks.len();
            let job_id_clone = job_id.clone();

            for (idx, task) in job.tasks.iter().enumerate() {
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

                let inner_c2 = inner.clone();
                let job_id_c2 = job_id_clone.clone();
                let item_update = move |name: String, status: TaskItemStatus, item_type: DownloadedItemType| {
                    let mut guard = inner_c2.lock().unwrap();
                    if let Some(j) = guard.jobs.iter_mut().find(|j| j.id == job_id_c2) {
                        if let Some(existing) = j.items.iter_mut().find(|i| i.name == name) {
                            existing.status = status;
                        } else {
                            j.items.push(TaskItemDetail {
                                name,
                                status,
                                item_type,
                            });
                        }
                    }
                };

                let context = TaskContext {
                    progress: &status_update,
                    item: &item_update,
                };

                let res = task.run(&context);

                if let Err(e) = res {
                    let err_msg = format!("Task failed: {}", e);
                    failed_task_errors.push(e);

                    let mut guard = inner.lock().unwrap();
                    if let Some(j) = guard.jobs.iter_mut().find(|j| j.id == job_id_clone) {
                        j.log.push(format!("[Error] {}", err_msg));
                    }
                    if let Some(tx) = guard.senders.get(&job_id_clone) {
                        let _ = tx.send(DownloadMsg::Error(err_msg));
                    }
                }
            }

            let job_failed = !failed_task_errors.is_empty();
            let job_err = failed_task_errors.join("; ");

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
    let progress_cb = move |msg, progress| {
        let _ = sender_clone.send(DownloadMsg::Progress(msg, progress));
    };
    let item_cb = |_, _, _| {};
    minecraft::download_minecraft_data_internal(
        version,
        loader,
        loader_version,
        data_path,
        &progress_cb,
        &item_cb,
    )
}

pub use java::{get_available_packages as fetch_java_packages, JavaDownloadProgress, JavaPackage};

pub use modrinth::{
    clear_caches as clear_modrinth_caches, get_project as fetch_modrinth_project,
    get_project_versions as fetch_modrinth_versions, search_mods as search_modrinth_mods,
    ModDependency, ModFile, ModProject, ModSearchResult, ModVersion,
};
