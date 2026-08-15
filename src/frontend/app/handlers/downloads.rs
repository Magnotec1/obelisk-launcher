use crate::backend::download::manager::DownloadMsg;
use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::RawVersion;
use crate::frontend::app::msg::AppMsg;
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::external::download::{
    DownloadDialogInput, DownloadState, DownloadStatusBarInput,
};
use crate::frontend::views::discover::DiscoverOutput;
use crate::frontend::views::instance::SummaryInput;
use adw::prelude::*;
use relm4::prelude::*;
use std::thread;

impl AppModel {
    pub(crate) fn handle_discover_event(
        &mut self,
        sender: &ComponentSender<AppModel>,
        out: DiscoverOutput,
    ) {
        match out {
            DiscoverOutput::InstallModpack(name, version_info, target_group, _provider) => {
                let version_info = *version_info;
                self.installing_modpack = true;
                if let Some(ref group_name) = target_group {
                    self.groups.set_instance_group(&name, group_name);
                    if let Some(ref path) = &self.config.instances_path {
                        let _ = self.groups.save(path);
                    }
                }
                self.download_status_bar.emit(DownloadStatusBarInput::Update(
                    DownloadState::Starting,
                    true,
                ));
                self.download_dialog
                    .emit(DownloadDialogInput::UpdateState(DownloadState::Starting));

                let sender_clone = sender.input_sender().clone();
                let instances_path = self.config.instances_path.clone();

                if let Some(path) = instances_path {
                    let job = crate::backend::download::manager::NetworkJob {
                        id: format!("modpack-{}", version_info.id),
                        title: format!("Installing Modpack {}", name),
                        tasks: vec![std::sync::Arc::new(
                            crate::backend::download::manager::ModrinthModpackDownloadTask {
                                name: name.clone(),
                                download_url: version_info.download_url.clone(),
                                instances_path: path.clone(),
                            },
                        )],
                        status: crate::backend::download::manager::NetworkJobStatus::Pending,
                        log: Vec::new(),
                        items: Vec::new(),
                    };

                    let (tx, rx) =
                        std::sync::mpsc::channel::<crate::backend::download::manager::DownloadMsg>();
                    crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);
                    crate::frontend::toast::show_toast(
                        &self.window,
                        format!("Started downloading Modpack: {}", name),
                    );

                    thread::spawn(move || {
                        while let Ok(msg) = rx.recv() {
                            let is_finished = matches!(
                                msg,
                                crate::backend::download::manager::DownloadMsg::Finished
                            );
                            let is_err = matches!(
                                msg,
                                crate::backend::download::manager::DownloadMsg::Error(_)
                            );

                            if sender_clone.send(AppMsg::DownloadProgress(msg)).is_err() {
                                break;
                            }
                            if is_finished {
                                let _ = sender_clone.send(AppMsg::RefreshInstances);
                                break;
                            }
                            if is_err {
                                break;
                            }
                        }
                    });
                } else {
                    crate::frontend::toast::show_toast(
                        &self.window,
                        "No instances directory configured",
                    );
                }
            }
            DiscoverOutput::DetailsOpened => {
                self.discover_details_open = true;
                if let Some(ref d) = self.discover_view.model().selected_details {
                    self.discover_details_title = d.info.title.clone();
                } else {
                    self.discover_details_title = "Modpack Details".to_string();
                }
            }
            DiscoverOutput::DetailsClosed => {
                self.discover_details_open = false;
                self.discover_details_title = String::new();
            }
        }
    }

    pub(crate) fn handle_download_start(
        &mut self,
        sender: &ComponentSender<AppModel>,
        raw_version: RawVersion,
        loader: ModLoader,
        loader_version: Option<String>,
    ) {
        self.download_dialog.emit(DownloadDialogInput::Start);
        self.download_status_bar
            .emit(DownloadStatusBarInput::Update(
                DownloadState::Starting,
                true,
            ));

        let data_path = self.config.minecraft_data_path.clone();
        let sender_clone = sender.input_sender().clone();

        let job = crate::backend::download::manager::NetworkJob {
            id: format!(
                "mc-{}-{}",
                raw_version.id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            title: format!("Minecraft {}", raw_version.id),
            tasks: vec![std::sync::Arc::new(
                crate::backend::download::manager::MinecraftDownloadTask {
                    version: raw_version.clone(),
                    loader: loader.clone(),
                    loader_version: loader_version.clone(),
                    data_path: data_path.clone(),
                },
            )],
            status: crate::backend::download::manager::NetworkJobStatus::Pending,
            log: Vec::new(),
            items: Vec::new(),
        };

        let (tx, rx) = std::sync::mpsc::channel::<crate::backend::download::manager::DownloadMsg>();

        crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);
        crate::frontend::toast::show_toast(
            &self.window,
            format!("Started downloading Minecraft {}", raw_version.id),
        );

        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let is_finished = matches!(
                    msg,
                    crate::backend::download::manager::DownloadMsg::Finished
                        | crate::backend::download::manager::DownloadMsg::Error(_)
                );
                let app_msg = AppMsg::DownloadProgress(msg);
                if sender_clone.send(app_msg).is_err() {
                    break;
                }
                if is_finished {
                    break;
                }
            }
        });
    }

    pub(crate) fn handle_download_progress(&mut self, sender: &ComponentSender<AppModel>, msg: DownloadMsg) {
        let (state, visible) = match msg {
            DownloadMsg::Progress(status, progress) => {
                let state = if progress == 0.0 {
                    DownloadState::Starting
                } else {
                    DownloadState::Downloading {
                        task: status,
                        current: 0,
                        total: 0,
                        item_name: String::new(),
                        progress,
                    }
                };
                (state, true)
            }
            DownloadMsg::DetailedProgress {
                task,
                current,
                total,
                item_name,
                overall_progress,
            } => {
                let state = DownloadState::Downloading {
                    task,
                    current,
                    total,
                    item_name,
                    progress: overall_progress,
                };
                (state, true)
            }
            DownloadMsg::Error(e) => {
                sender.input(AppMsg::DownloadError(e));
                return;
            }
            DownloadMsg::Finished => {
                sender.input(AppMsg::DownloadFinished);
                return;
            }
        };

        self.download_dialog
            .emit(DownloadDialogInput::UpdateState(state.clone()));
        self.download_status_bar
            .emit(DownloadStatusBarInput::Update(state, visible));
    }

    pub(crate) fn handle_download_finished(&mut self, sender: &ComponentSender<AppModel>) {
        if self.verifying_loading {
            crate::frontend::toast::show_toast(
                &self.window,
                "Instance verification completed successfully!",
            );
        }
        if self.installing_modpack {
            crate::frontend::toast::show_toast(&self.window, "Modpack installed successfully!");
            self.installing_modpack = false;
        }
        self.verifying_loading = false;
        self.instance_summary
            .emit(SummaryInput::SetVerifyingLoading(false));

        let has_active_jobs = crate::backend::download::manager::DOWNLOAD_QUEUE
            .get_jobs()
            .into_iter()
            .any(|j| {
                matches!(
                    j.status,
                    crate::backend::download::manager::NetworkJobStatus::Pending
                        | crate::backend::download::manager::NetworkJobStatus::Running { .. }
                )
            });

        if !has_active_jobs {
            self.download_dialog
                .emit(DownloadDialogInput::UpdateState(DownloadState::Finished));
            self.download_status_bar
                .emit(DownloadStatusBarInput::Update(
                    DownloadState::Finished,
                    false,
                ));
        } else {
            self.download_dialog.emit(DownloadDialogInput::Refresh);
        }

        if self.launch_after_download {
            self.launch_after_download = false;
            sender.input(AppMsg::LaunchInstance);
        }
    }

    pub(crate) fn handle_download_error(&mut self, err: String) {
        self.launch_after_download = false;
        self.verifying_loading = false;
        self.installing_modpack = false;
        self.instance_summary
            .emit(SummaryInput::SetVerifyingLoading(false));

        let has_active_jobs = crate::backend::download::manager::DOWNLOAD_QUEUE
            .get_jobs()
            .into_iter()
            .any(|j| {
                matches!(
                    j.status,
                    crate::backend::download::manager::NetworkJobStatus::Pending
                        | crate::backend::download::manager::NetworkJobStatus::Running { .. }
                )
            });

        if !has_active_jobs {
            self.download_dialog
                .emit(DownloadDialogInput::UpdateState(DownloadState::Failed(
                    err.clone(),
                )));
            self.download_status_bar
                .emit(DownloadStatusBarInput::Update(
                    DownloadState::Failed(err),
                    true,
                ));
        } else {
            self.download_dialog.emit(DownloadDialogInput::Refresh);
        }
    }

    pub(crate) fn handle_dismiss_download_status(&self) {
        self.download_status_bar
            .emit(DownloadStatusBarInput::Dismiss);
    }

    pub(crate) fn handle_remove_job(&self, id: String) {
        crate::backend::download::manager::DOWNLOAD_QUEUE.remove_job(&id);
        self.download_dialog.emit(DownloadDialogInput::Refresh);
    }

    pub(crate) fn handle_clear_finished_jobs(&self) {
        crate::backend::download::manager::DOWNLOAD_QUEUE.clear_finished_jobs();
        self.download_dialog.emit(DownloadDialogInput::Refresh);
    }

    pub(crate) fn handle_retry_job(&self, sender: &ComponentSender<AppModel>, id: String) {
        let (tx, rx) = std::sync::mpsc::channel::<crate::backend::download::manager::DownloadMsg>();
        crate::backend::download::manager::DOWNLOAD_QUEUE.retry_job(&id, tx);
        self.download_dialog.emit(DownloadDialogInput::Refresh);

        let sender_clone = sender.input_sender().clone();
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let is_finished = matches!(
                    msg,
                    crate::backend::download::manager::DownloadMsg::Finished
                        | crate::backend::download::manager::DownloadMsg::Error(_)
                );
                let app_msg = AppMsg::DownloadProgress(msg);
                if sender_clone.send(app_msg).is_err() {
                    break;
                }
                if is_finished {
                    break;
                }
            }
        });
    }

    pub(crate) fn handle_open_download_details(&self) {
        self.download_dialog.emit(DownloadDialogInput::Show);
        self.download_dialog.widget().present(Some(&self.window));
    }
}
