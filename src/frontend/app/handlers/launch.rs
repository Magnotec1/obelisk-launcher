use crate::backend::auth::account::get_active_account;
use crate::backend::instance::launcher::{check_instance_assets, launch_instance, LaunchOptions};
use crate::backend::instance::manager::scan_single_instance;
use crate::backend::playtime::PlaySession;
use crate::backend::runtime::versions::find_version_by_id;
use crate::frontend::app::msg::{AppMsg, InstanceStatus};
use crate::frontend::app::state::AppModel;
use crate::frontend::views::instance::{ConsoleInput, SummaryInput};
use chrono::Utc;
use gtk::prelude::*;
use relm4::prelude::*;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::thread;

impl AppModel {
    pub(crate) fn handle_set_instance_feral_game_mode(&mut self, enabled: bool) {
        if let Some(path) = self.get_active_instance_path() {
            let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                &path,
                "FeralGameMode",
                enabled,
            );
            if let Some(idx) = self.selected_instance {
                if let Some(inst) = self.instances.get_mut(idx) {
                    inst.feral_gamemode = enabled;
                }
            }
        }
    }

    pub(crate) fn handle_set_instance_discrete_gpu(&mut self, enabled: bool) {
        if let Some(path) = self.get_active_instance_path() {
            let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                &path,
                "DiscreteGpu",
                enabled,
            );
            if let Some(idx) = self.selected_instance {
                if let Some(inst) = self.instances.get_mut(idx) {
                    inst.discrete_gpu = enabled;
                }
            }
        }
    }

    pub(crate) fn handle_set_instance_zink_vulkan(&mut self, enabled: bool) {
        if let Some(path) = self.get_active_instance_path() {
            let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                &path,
                "ZinkVulkan",
                enabled,
            );
            if let Some(idx) = self.selected_instance {
                if let Some(inst) = self.instances.get_mut(idx) {
                    inst.zink_vulkan = enabled;
                }
            }
        }
    }

    pub(crate) fn handle_set_instance_use_wayland(&mut self, enabled: bool) {
        if let Some(path) = self.get_active_instance_path() {
            let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                &path,
                "UseWayland",
                enabled,
            );
            if let Some(idx) = self.selected_instance {
                if let Some(inst) = self.instances.get_mut(idx) {
                    inst.use_wayland = enabled;
                }
            }
        }
    }

    pub(crate) fn handle_kill_instance(&mut self, sender: &ComponentSender<AppModel>) {
        if let Some(idx) = self.selected_instance {
            sender.input(AppMsg::KillInstanceFromIndex(idx));
        }
    }

    pub(crate) fn handle_kill_instance_from_index(&mut self, idx: usize) {
        if let Some(inst) = self.instances.get(idx) {
            let path = inst.path.clone();
            let game_process = self.get_game_process(&path);
            let mut guard = game_process.lock().unwrap();
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                self.instance_statuses
                    .insert(path.clone(), InstanceStatus::NotRunning);
                self.rebuild_overview();

                if Some(&path) == self.get_active_instance_path().as_ref() {
                    let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                    self.instance_summary
                        .emit(SummaryInput::Update(Box::new(inst), InstanceStatus::NotRunning));
                }

                let buf = self.get_console_buffer(&path);
                let mut iter = buf.end_iter();
                buf.insert(&mut iter, "\nProcess killed by user.\n");
            }
        }
    }

    pub(crate) fn handle_verify_instance(&mut self, sender: &ComponentSender<AppModel>) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            let instance = inst.clone();
            self.launch_after_download = false;
            self.verifying_loading = true;
            self.instance_summary
                .emit(SummaryInput::SetVerifyingLoading(true));
            let sender_clone = sender.input_sender().clone();
            crate::backend::core::tasks::spawn_io(move || {
                if let Some(mc_version) = &instance.minecraft_version {
                    match find_version_by_id(mc_version) {
                        Ok(Some(v)) => {
                            let (loader, loader_ver) = instance.get_loader_info();
                            let _ = sender_clone
                                .send(AppMsg::DownloadStart(v.raw, loader, loader_ver));
                        }
                        _ => {
                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance.path.clone(),
                                format!(
                                    "Could not resolve Minecraft version {} for verify.\n",
                                    mc_version
                                ),
                            ));
                            let _ = sender_clone
                                .send(AppMsg::DownloadError("Version resolution failed".into()));
                        }
                    }
                }
            });
        }
    }

    pub(crate) fn handle_launch_instance(&mut self, sender: &ComponentSender<AppModel>) {
        if let Some(idx) = self.selected_instance {
            sender.input(AppMsg::LaunchInstanceFromIndex(idx));
        }
    }

    pub(crate) fn handle_launch_instance_from_index(
        &mut self,
        sender: &ComponentSender<AppModel>,
        idx: usize,
    ) {
        if let Some(inst) = self.instances.get(idx) {
            let instance = inst.clone();
            let status = self.get_instance_status(&instance.path);
            if status != InstanceStatus::NotRunning {
                return;
            }

            let mut options = LaunchOptions::default();
            if let Some(shared_path) = self.config.shared_data_path.clone() {
                options.shared_data_path = shared_path;
            }
            options.mc_data_path = self.config.minecraft_data_path.clone();

            let is_java_automatic = inst.java_path.is_none();
            let java_path = inst
                .java_path
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from("java"));

            options.java_path = java_path;
            options.is_java_automatic = is_java_automatic;
            options.max_memory = self.config.max_memory;
            options.min_memory = self.config.min_memory;

            // Auto-refresh active Microsoft account token if expired or expiring soon (< 1 hour)
            if let Some(active_account) = get_active_account(&self.config) {
                if active_account.account_type == crate::backend::auth::microsoft::AccountType::Microsoft {
                    let status = crate::backend::auth::account::verify_account_status(active_account);
                    if matches!(status, crate::backend::auth::account::AccountStatus::Expired | crate::backend::auth::account::AccountStatus::ExpiringSoon) {
                        let client_id = self.config.microsoft_client_id.clone().unwrap_or_else(|| "00000000402b5328".to_string());
                        if let Ok(refreshed) = crate::backend::auth::account::refresh_single_account(active_account, &client_id) {
                            crate::backend::auth::account::add_account(&mut self.config, refreshed);
                            let _ = self.config.save();
                            self.account_view.emit(crate::frontend::views::account::AccountInput::UpdateConfig(self.config.clone()));
                        }
                    }
                }
            }

            options.account = get_active_account(&self.config).cloned();

            if !check_instance_assets(&instance, &options) {
                sender.input(AppMsg::SelectInstance(idx));
                self.launch_after_download = true;
                let sender_clone = sender.input_sender().clone();
                crate::backend::core::tasks::spawn_io(move || {
                    if let Some(mc_version) = &instance.minecraft_version {
                        match find_version_by_id(mc_version) {
                            Ok(Some(v)) => {
                                let (loader, loader_ver) = instance.get_loader_info();
                                let _ = sender_clone
                                    .send(AppMsg::DownloadStart(v.raw, loader, loader_ver));
                            }
                            _ => {
                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance.path.clone(),
                                    format!(
                                        "Could not resolve Minecraft version {} for download.\n",
                                        mc_version
                                    ),
                                ));
                                let _ = sender_clone
                                    .send(AppMsg::DownloadError("Version resolution failed".into()));
                            }
                        }
                    }
                });
                return;
            }

            self.instance_statuses
                .insert(instance.path.clone(), InstanceStatus::Loading);
            self.rebuild_overview();

            if Some(idx) == self.selected_instance {
                self.instance_summary.emit(SummaryInput::Update(
                    Box::new(Some(instance.clone())),
                    InstanceStatus::Loading,
                ));
            }

            let buf = self.get_console_buffer(&instance.path);
            buf.set_text(&format!("Starting launch for {}...\n", instance.name));

            let sender_clone = sender.input_sender().clone();
            let game_process = self.get_game_process(&instance.path);
            let instance_path = instance.path.clone();
            thread::spawn(move || {
                let start_time_chrono = Utc::now();
                let start_time = std::time::Instant::now();
                let mut options = options;
                let max_mem = options.max_memory;
                let min_mem = options.min_memory;

                let is_flatpak = std::path::Path::new("/.flatpak-info").exists()
                    || std::env::var("FLATPAK_ID").is_ok();

                let mc_ver = instance.minecraft_version.as_deref().unwrap_or("1.20.1");
                let required_ver = crate::backend::runtime::java::get_required_java_version(mc_ver);

                let selected_probed = if options.is_java_automatic {
                    let launcher_dir = options.mc_data_path.join("java");
                    let available = crate::backend::runtime::java::find_java_versions(Some(&launcher_dir));
                    let matching = available.into_iter().find(|j| {
                        crate::backend::runtime::java::get_java_major_version(&j.version) == Some(required_ver)
                    });
                    match matching {
                        Some(j) => {
                            options.java_path = j.path.clone();
                            Some(j)
                        }
                        None => {
                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance_path.clone(),
                                format!("No suitable Java {} found automatically. Please install it or set a custom Java path in instance settings.\n", required_ver),
                            ));
                            let _ = sender_clone.send(AppMsg::ProcessFinished(
                                instance_path.clone(),
                                0,
                                start_time_chrono,
                                Utc::now(),
                            ));
                            return;
                        }
                    }
                } else {
                    crate::backend::runtime::java::probe_java(&options.java_path)
                };

                let _ = sender_clone.send(AppMsg::ConsoleLog(
                    instance_path.clone(),
                    format!("Launching instance '{}'...\n", instance.name),
                ));

                match launch_instance(&instance, options.clone()) {
                    Ok(mut child) => {
                        let stdout = child.stdout.take();
                        let stderr = child.stderr.take();

                        {
                            let mut guard = game_process.lock().unwrap();
                            *guard = Some(child);
                        }

                        let _ = sender_clone.send(AppMsg::InstanceLaunched(
                            instance_path.clone(),
                            start_time_chrono.timestamp() as u64,
                        ));

                        let s_clone = sender_clone.clone();
                        let ip_clone = instance_path.clone();
                        if let Some(out) = stdout {
                            thread::spawn(move || {
                                let reader = BufReader::new(out);
                                for line in reader.lines().map_while(Result::ok) {
                                    let _ = s_clone
                                        .send(AppMsg::ConsoleLog(ip_clone.clone(), format!("{}\n", line)));
                                }
                            });
                        }

                        let s_clone_err = sender_clone.clone();
                        let ip_clone_err = instance_path.clone();
                        if let Some(err) = stderr {
                            thread::spawn(move || {
                                let reader = BufReader::new(err);
                                for line in reader.lines().map_while(Result::ok) {
                                    let _ = s_clone_err.send(AppMsg::ConsoleLog(
                                        ip_clone_err.clone(),
                                        format!("{}\n", line),
                                    ));
                                }
                            });
                        }

                        loop {
                            let is_done = {
                                let mut guard = game_process.lock().unwrap();
                                if let Some(ref mut c) = *guard {
                                    match c.try_wait() {
                                        Ok(Some(status)) => Some(status),
                                        Ok(None) => None,
                                        Err(_) => Some(std::process::ExitStatus::default()),
                                    }
                                } else {
                                    Some(std::process::ExitStatus::default())
                                }
                            };

                            if let Some(exit_status) = is_done {
                                let duration = start_time.elapsed().as_secs();
                                let mut guard = game_process.lock().unwrap();
                                *guard = None;

                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance_path.clone(),
                                    format!("\nProcess exited with status: {}\n", exit_status),
                                ));

                                if !exit_status.success() {
                                    let flatpak_tip = if is_flatpak {
                                        "\nNote: You are running Obelisk in Flatpak. Ensure Java paths are accessible to the sandbox (e.g. within ~/.var/app/ or system runtime)."
                                    } else {
                                        ""
                                    };

                                    let diagnosis = format!(
                                        "== Crash / Launch Failure Diagnosis ==\n• Required Java Version: Java {}\n• Selected Java Path: {}\n• Detected Java Version: {}\n• Compatibility Check: {}\n• Memory Allocation: Max {}MB, Min {}MB{}\n======================================\n",
                                        required_ver,
                                        options.java_path.display(),
                                        selected_probed.as_ref().map(|j| j.version.as_str()).unwrap_or("Unknown"),
                                        if selected_probed.is_some() && crate::backend::runtime::java::get_java_major_version(&selected_probed.as_ref().unwrap().version).unwrap_or(0) == required_ver {
                                            "Java version seems correct."
                                        } else {
                                            "Java version seems incorrect! Try installing and using another java version."
                                        },
                                        max_mem,
                                        min_mem,
                                        flatpak_tip
                                    );

                                    let _ = sender_clone
                                        .send(AppMsg::ConsoleLog(instance_path.clone(), diagnosis));
                                }

                                let _ = sender_clone.send(AppMsg::ProcessFinished(
                                    instance_path.clone(),
                                    duration,
                                    start_time_chrono,
                                    Utc::now(),
                                ));
                                break;
                            }
                            thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                    Err(e) => {
                        let _ = sender_clone.send(AppMsg::ConsoleLog(
                            instance_path.clone(),
                            format!("Launch failed: {}\n", e),
                        ));
                        let _ = sender_clone.send(AppMsg::ProcessFinished(
                            instance_path.clone(),
                            0,
                            start_time_chrono,
                            Utc::now(),
                        ));
                    }
                }
            });
        }
    }

    pub(crate) fn handle_console_log(&mut self, path: PathBuf, msg: String) {
        let is_active = Some(&path) == self.get_active_instance_path().as_ref();

        self.instance_logs
            .entry(path.clone())
            .or_default()
            .push(msg.clone());

        let query = self.console_search_query.to_lowercase();
        if query.is_empty() || msg.to_lowercase().contains(&query) {
            let buf = self.get_console_buffer(&path);
            let mut iter = buf.end_iter();
            buf.insert(&mut iter, &msg);

            if is_active {
                let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                let status = self.get_instance_status(&path);
                self.instance_summary
                    .emit(SummaryInput::Update(Box::new(inst), status));
                self.instance_console.emit(ConsoleInput::Update {
                    buffer: buf,
                    status,
                    has_any_logs: self.get_active_instance_has_logs(),
                });
            }
        }
    }

    pub(crate) fn handle_process_finished(
        &mut self,
        sender: &ComponentSender<AppModel>,
        path: PathBuf,
        duration: u64,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) {
        let is_active = Some(&path) == self.get_active_instance_path().as_ref();
        self.instance_statuses.remove(&path);

        if duration > 0 {
            let _ = crate::backend::instance::manager::update_instance_playtime(&path, duration);
        }

        let mut found = false;
        let mut instance_id = String::new();
        for inst in &mut self.instances {
            if inst.path == path {
                inst.total_time_played += duration;
                instance_id = inst.id.clone();
                found = true;
                break;
            }
        }

        if found && duration > 0 {
            if !instance_id.is_empty() {
                self.playtime_manager.add_session(PlaySession {
                    instance_id: instance_id.clone(),
                    start_time: start,
                    end_time: end,
                    duration_seconds: duration,
                });
            }

            self.config.total_playtime += duration;
            let _ = self.config.save();
        }
        self.rebuild_overview();

        if is_active {
            let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
            self.instance_summary
                .emit(SummaryInput::Update(Box::new(inst), InstanceStatus::NotRunning));
            self.instance_console.emit(ConsoleInput::Update {
                buffer: self.get_active_console_buffer(),
                status: InstanceStatus::NotRunning,
                has_any_logs: self.get_active_instance_has_logs(),
            });

            let sender_clone = sender.input_sender().clone();
            let path_clone = path.clone();
            crate::backend::core::tasks::spawn_io(move || {
                if let Some(inst) = scan_single_instance(&path_clone, true) {
                    let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(inst));
                }
            });
        }
    }

    pub(crate) fn handle_clear_console(&mut self, path: PathBuf) {
        if let Some(logs) = self.instance_logs.get_mut(&path) {
            logs.clear();
        }
        self.rebuild_console_buffer(&path);
        if Some(&path) == self.get_active_instance_path().as_ref() {
            self.instance_console.emit(ConsoleInput::Update {
                buffer: self.get_active_console_buffer(),
                status: self.get_active_instance_status(),
                has_any_logs: self.get_active_instance_has_logs(),
            });
        }
    }

    pub(crate) fn handle_set_console_search_query(&mut self, query: String) {
        self.console_search_query = query;
        if let Some(path) = self.get_active_instance_path() {
            self.rebuild_console_buffer(&path);
            self.instance_console.emit(ConsoleInput::Update {
                buffer: self.get_active_console_buffer(),
                status: self.get_active_instance_status(),
                has_any_logs: self.get_active_instance_has_logs(),
            });
        }
    }

    pub(crate) fn handle_instance_launched(&mut self, path: PathBuf, timestamp: u64) {
        self.instance_statuses
            .insert(path.clone(), InstanceStatus::Running);
        for inst in &mut self.instances {
            if inst.path == path {
                inst.last_launched = Some(timestamp);
                break;
            }
        }
        self.rebuild_overview();
        if Some(&path) == self.get_active_instance_path().as_ref() {
            let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
            self.instance_summary
                .emit(SummaryInput::Update(Box::new(inst), InstanceStatus::Running));
            self.instance_console.emit(ConsoleInput::Update {
                buffer: self.get_active_console_buffer(),
                status: InstanceStatus::Running,
                has_any_logs: self.get_active_instance_has_logs(),
            });
        }
    }
}
