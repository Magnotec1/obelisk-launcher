use crate::backend::instance::manager::{
    add_instance_item, is_loader_component, remove_component, remove_instance_item,
    remove_mod_loader, set_component_version, set_instance_java, set_mod_loader_with_version,
    ModLoader,
};
use crate::frontend::app::msg::AppMsg;
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::external::browser::BrowserInput;
use crate::frontend::dialogs::external::download::{DownloadState, DownloadStatusBarInput};
use crate::frontend::dialogs::instance::components::{
    ComponentEditorInput, ComponentEditorOutput,
};
use crate::frontend::dialogs::instance::editor::{
    EditorInput, EditorItem, EditorOutput, EditorType, ModUpdateInfo,
};
use crate::frontend::dialogs::instance::mod_loader::{
    ModLoaderDialogInput, ModLoaderDialogOutput,
};
use crate::frontend::dialogs::system::java::JavaSelectorInput;
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

impl AppModel {
    pub(crate) fn handle_edit_components(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.active_editor_type = Some(EditorType::Components);
            let items = inst
                .components
                .iter()
                .map(|c| EditorItem {
                    id: c.uid.clone(),
                    name: c.name.clone(),
                    version: c.version.clone(),
                    filename: c.uid.clone(),
                    description: None,
                    homepage: None,
                    sources: None,
                    icon_path: None,
                    is_checked: false,
                    size: None,
                    seed: None,
                    last_played: None,
                    enabled: true,
                })
                .collect();
            self.instance_editor.emit(EditorInput::Open(
                EditorType::Components,
                "Edit Components".to_string(),
                items,
            ));
            self.instance_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_edit_mods(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.active_editor_type = Some(EditorType::Mods);
            let items = inst
                .mods
                .iter()
                .map(|m| EditorItem {
                    id: m.filename.clone(),
                    name: m.name.clone(),
                    version: m.version.clone(),
                    filename: m.filename.clone(),
                    description: m.description.clone(),
                    homepage: m.homepage.clone(),
                    sources: None,
                    icon_path: m.icon_path.clone(),
                    is_checked: false,
                    size: None,
                    seed: None,
                    last_played: None,
                    enabled: m.enabled,
                })
                .collect();
            self.instance_editor.emit(EditorInput::Open(
                EditorType::Mods,
                "Edit Mods".to_string(),
                items,
            ));
            self.instance_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_edit_resource_packs(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.active_editor_type = Some(EditorType::ResourcePacks);
            let items = inst
                .resource_packs
                .iter()
                .map(|rp| EditorItem {
                    id: rp.filename.clone(),
                    name: rp.name.clone(),
                    version: rp
                        .format
                        .map(|f| format!("Format {}", f))
                        .unwrap_or_default(),
                    filename: rp.filename.clone(),
                    description: rp.description.clone(),
                    homepage: None,
                    sources: None,
                    icon_path: rp.icon_path.clone(),
                    is_checked: false,
                    size: Some(crate::frontend::utils::format_size(rp.size)),
                    seed: None,
                    last_played: None,
                    enabled: true,
                })
                .collect();
            self.instance_editor.emit(EditorInput::Open(
                EditorType::ResourcePacks,
                "Edit Resource Packs".to_string(),
                items,
            ));
            self.instance_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_edit_shader_packs(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.active_editor_type = Some(EditorType::ShaderPacks);
            let items = inst
                .shader_packs
                .iter()
                .map(|sp| EditorItem {
                    id: sp.filename.clone(),
                    name: sp.name.clone(),
                    version: String::new(),
                    filename: sp.filename.clone(),
                    description: sp.description.clone(),
                    homepage: None,
                    sources: None,
                    icon_path: sp.icon_path.clone(),
                    is_checked: false,
                    size: Some(crate::frontend::utils::format_size(sp.size)),
                    seed: None,
                    last_played: None,
                    enabled: true,
                })
                .collect();
            self.instance_editor.emit(EditorInput::Open(
                EditorType::ShaderPacks,
                "Edit Shader Packs".to_string(),
                items,
            ));
            self.instance_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_edit_worlds(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.active_editor_type = Some(EditorType::Worlds);
            let items = inst
                .worlds
                .iter()
                .map(|w| {
                    let size_str = crate::frontend::utils::format_size(w.file_size);
                    EditorItem {
                        id: w.folder_name.clone(),
                        name: w.name.clone(),
                        version: w.mc_version.clone().unwrap_or_default(),
                        filename: w.folder_name.clone(),
                        description: None,
                        homepage: None,
                        sources: None,
                        icon_path: None,
                        is_checked: false,
                        size: Some(size_str),
                        seed: w.seed.map(|s| s.to_string()),
                        last_played: w
                            .last_played
                            .map(crate::frontend::utils::format_timestamp),
                        enabled: true,
                    }
                })
                .collect();
            self.instance_editor.emit(EditorInput::Open(
                EditorType::Worlds,
                "Edit Worlds".to_string(),
                items,
            ));
            self.instance_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_open_mods_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "mods");
        }
    }

    pub(crate) fn handle_open_resource_packs_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::open_instance_subfolder(
                &inst.minecraft_dir,
                "resourcepacks",
            );
        }
    }

    pub(crate) fn handle_open_shader_packs_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::open_instance_subfolder(
                &inst.minecraft_dir,
                "shaderpacks",
            );
        }
    }

    pub(crate) fn handle_open_screenshots_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::open_instance_subfolder(
                &inst.minecraft_dir,
                "screenshots",
            );
        }
    }

    pub(crate) fn handle_open_worlds_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "saves");
        }
    }

    pub(crate) fn handle_browse_modrinth(&mut self, editor_type: EditorType) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            let (loader, _) = inst.get_loader_info();
            self.browser_dialog.emit(BrowserInput::Open {
                game_version: inst.minecraft_version.clone().unwrap_or_default(),
                loader,
                editor_type,
            });
            self.browser_dialog.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_open_java_selector(&mut self) {
        self.java_selector.emit(JavaSelectorInput::Open);
        self.java_selector.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_set_instance_java(&self, sender: &ComponentSender<AppModel>, path: PathBuf) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                let _ = set_instance_java(&inst.path, &path);
                sender.input(AppMsg::RefreshSelectedInstance);
            }
        }
    }

    pub(crate) fn handle_set_instance_java_default(&self, sender: &ComponentSender<AppModel>) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                let _ = crate::backend::instance::manager::remove_instance_java(&inst.path);
                sender.input(AppMsg::RefreshSelectedInstance);
            }
        }
    }

    pub(crate) fn handle_open_component_swap(&mut self, uid: String) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            let current_version = inst
                .components
                .iter()
                .find(|c| c.uid == uid)
                .map(|c| c.version.clone());

            self.component_editor.emit(ComponentEditorInput::Open(
                uid,
                inst.minecraft_version.clone(),
                current_version,
            ));
            self.component_editor.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_remove_component(&mut self, sender: &ComponentSender<AppModel>, uid: String) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                if is_loader_component(&uid) {
                    let _ = remove_mod_loader(&inst.path);
                    crate::frontend::toast::show_toast(&self.window, "Removed mod loader");
                } else {
                    let _ = remove_component(&inst.path, &uid);
                    crate::frontend::toast::show_toast(&self.window, "Removed component");
                }
                sender.input(AppMsg::RefreshSelectedInstance);
            }
        }
    }

    pub(crate) fn handle_select_mod_loader_request(&mut self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            self.mod_loader_dialog
                .emit(ModLoaderDialogInput::Open(inst.minecraft_version.clone()));
            self.mod_loader_dialog.widget().present(Some(&self.window));
        }
    }

    pub(crate) fn handle_mod_loader_output(
        &mut self,
        sender: &ComponentSender<AppModel>,
        output: ModLoaderDialogOutput,
    ) {
        match output {
            ModLoaderDialogOutput::InstallModLoader(loader, version) => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let _ = set_mod_loader_with_version(&inst.path, &loader, &version);
                        crate::frontend::toast::show_toast(
                            &self.window,
                            format!("Installed {:?} {}", loader, version),
                        );
                        sender.input(AppMsg::RefreshSelectedInstance);
                    }
                }
            }
        }
    }

    pub(crate) fn handle_component_editor_output(
        &mut self,
        sender: &ComponentSender<AppModel>,
        output: ComponentEditorOutput,
    ) {
        match output {
            ComponentEditorOutput::SetVersion(uid, version) => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let _ = set_component_version(&inst.path, &uid, &version);
                        crate::frontend::toast::show_toast(
                            &self.window,
                            format!("Updated version to {}", version),
                        );
                        sender.input(AppMsg::RefreshSelectedInstance);
                    }
                }
            }
        }
    }

    pub(crate) fn handle_install_browser_items(
        &mut self,
        sender: &ComponentSender<AppModel>,
        editor_type: EditorType,
        installs: Vec<(String, String)>,
    ) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                let gv = inst
                    .minecraft_version
                    .clone()
                    .unwrap_or_else(|| "1.20.1".to_string());
                let (loader, _) = inst.get_loader_info();

                let et_clone = editor_type.clone();
                let minecraft_dir = inst.minecraft_dir.clone();
                let sender_clone = sender.input_sender().clone();

                let target_dir = match editor_type {
                    EditorType::Mods => minecraft_dir.join("mods"),
                    EditorType::ResourcePacks => minecraft_dir.join("resourcepacks"),
                    EditorType::ShaderPacks => minecraft_dir.join("shaderpacks"),
                    EditorType::Worlds => minecraft_dir.join("saves"),
                    _ => minecraft_dir.join("mods"),
                };
                if !target_dir.exists() {
                    let _ = std::fs::create_dir_all(&target_dir);
                }

                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        DownloadState::Starting,
                        true,
                    ));

                let mut tasks = Vec::new();
                let installs_len = installs.len();
                for (project_id, version_id) in installs {
                    tasks.push(
                        std::sync::Arc::new(crate::backend::download::manager::ModrinthDownloadTask {
                            project_id,
                            version_id: if version_id.is_empty() {
                                None
                            } else {
                                Some(version_id)
                            },
                            game_version: gv.clone(),
                            loader: if matches!(editor_type, EditorType::Mods) {
                                loader.clone()
                            } else {
                                ModLoader::None
                            },
                            mods_dir: target_dir.clone(),
                            old_filename: None,
                        }) as std::sync::Arc<dyn crate::backend::download::manager::DownloadTask>,
                    );
                }

                let item_label = match editor_type {
                    EditorType::Mods => "Mods",
                    EditorType::ResourcePacks => "Resource Packs",
                    EditorType::ShaderPacks => "Shader Packs",
                    EditorType::Worlds => "Worlds",
                    _ => "Items",
                };

                let job = crate::backend::download::manager::NetworkJob {
                    id: format!(
                        "browser-{}-{}",
                        item_label.to_lowercase().replace(' ', "-"),
                        uuid::Uuid::new_v4()
                    ),
                    title: format!("{} for {}", item_label, inst.name),
                    tasks,
                    status: crate::backend::download::manager::NetworkJobStatus::Pending,
                    log: Vec::new(),
                    items: Vec::new(),
                };

                let (tx, rx) = std::sync::mpsc::channel::<
                    crate::backend::download::manager::DownloadMsg,
                >();

                crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);

                crate::backend::core::tasks::spawn_io(move || {
                    let mut success_count = 0;
                    let mut last_error = None;

                    while let Ok(msg) = rx.recv() {
                        let is_finished = matches!(
                            msg,
                            crate::backend::download::manager::DownloadMsg::Finished
                                | crate::backend::download::manager::DownloadMsg::Error(_)
                        );

                        match &msg {
                            crate::backend::download::manager::DownloadMsg::Finished => {
                                success_count = installs_len;
                            }
                            crate::backend::download::manager::DownloadMsg::Error(err) => {
                                last_error = Some(err.clone());
                            }
                            _ => {}
                        }

                        let app_msg = AppMsg::DownloadProgress(msg);
                        if sender_clone.send(app_msg).is_err() {
                            break;
                        }
                        if is_finished {
                            break;
                        }
                    }

                    if success_count > 0 {
                        sender_clone
                            .send(AppMsg::ModrinthInstallResult(et_clone, Ok(success_count)))
                            .ok();
                        sender_clone.send(AppMsg::RefreshSelectedInstance).ok();
                    } else if let Some(e) = last_error {
                        sender_clone
                            .send(AppMsg::ModrinthInstallResult(et_clone, Err(e)))
                            .ok();
                    }
                });
            }
        }
    }

    pub(crate) fn handle_modrinth_install_result(
        &mut self,
        editor_type: EditorType,
        result: Result<usize, String>,
    ) {
        let item_label = match editor_type {
            EditorType::Mods => "mod",
            EditorType::ResourcePacks => "resource pack",
            EditorType::ShaderPacks => "shader pack",
            EditorType::Worlds => "world",
            _ => "item",
        };
        let items_label = match editor_type {
            EditorType::Mods => "mods",
            EditorType::ResourcePacks => "resource packs",
            EditorType::ShaderPacks => "shader packs",
            EditorType::Worlds => "worlds",
            _ => "items",
        };
        match result {
            Ok(count) => {
                let msg = if count == 1 {
                    format!("Successfully installed 1 {}", item_label)
                } else {
                    format!("Successfully installed {} {}", count, items_label)
                };
                self.instance_editor.emit(EditorInput::ShowToast(msg));
            }
            Err(e) => {
                self.instance_editor.emit(EditorInput::ShowToast(format!(
                    "Installation failed: {}",
                    e
                )));
            }
        }
    }

    pub(crate) fn handle_mod_updates_result(&mut self, result: Result<Vec<ModUpdateInfo>, String>) {
        match result {
            Ok(updates) => {
                self.instance_editor
                    .emit(EditorInput::UpdatesAvailable(updates));
                crate::frontend::toast::show_toast(&self.window, "Mod update check complete.");
            }
            Err(e) => {
                crate::frontend::toast::show_toast(
                    &self.window,
                    format!("Failed to check for updates: {}", e),
                );
            }
        }
    }

    pub(crate) fn handle_mod_update_success(&mut self, filename: String) {
        self.instance_editor
            .emit(EditorInput::UpdateSuccess(filename));
    }

    pub(crate) fn handle_mod_update_all_success(&mut self, filenames: Vec<String>) {
        self.instance_editor
            .emit(EditorInput::UpdateAllSuccess(filenames));
    }

    pub(crate) fn handle_editor_output_action(
        &mut self,
        sender: &ComponentSender<AppModel>,
        output: EditorOutput,
    ) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                match output {
                    EditorOutput::SetModsEnabled(ids, enabled) => {
                        for id in ids {
                            let _ = crate::backend::instance::manager::toggle_mod_enabled(
                                &inst.path, &id, enabled,
                            );
                        }
                    }
                    EditorOutput::RemoveMods(ids) => {
                        for id in ids {
                            let _ = crate::backend::instance::manager::remove_instance_item(
                                &inst.path, "mods", &id,
                            );
                        }
                    }
                    EditorOutput::RemoveComponents(ids) => {
                        for id in ids {
                            if crate::backend::instance::manager::is_loader_component(&id) {
                                let _ = crate::backend::instance::manager::remove_mod_loader(&inst.path);
                            } else {
                                let _ = crate::backend::instance::manager::remove_component(&inst.path, &id);
                            }
                        }
                    }
                    EditorOutput::RemoveResourcePacks(ids) => {
                        for id in ids {
                            let _ = crate::backend::instance::manager::remove_instance_item(
                                &inst.path, "resourcepacks", &id,
                            );
                        }
                    }
                    EditorOutput::RemoveShaderPacks(ids) => {
                        for id in ids {
                            let _ = crate::backend::instance::manager::remove_instance_item(
                                &inst.path, "shaderpacks", &id,
                            );
                        }
                    }
                    EditorOutput::RemoveWorlds(ids) => {
                        for id in ids {
                            let _ = crate::backend::instance::manager::remove_instance_item(
                                &inst.path, "saves", &id,
                            );
                        }
                    }
                    EditorOutput::AddItems(editor_type, paths) => {
                        let (subfolder, label) = match editor_type {
                            EditorType::Mods => ("mods", "mod(s)"),
                            EditorType::ResourcePacks => ("resourcepacks", "resource pack(s)"),
                            EditorType::ShaderPacks => ("shaderpacks", "shader pack(s)"),
                            EditorType::Worlds => ("saves", "world(s)"),
                            _ => ("mods", "item(s)"),
                        };
                        let mut count = 0;
                        for path in paths {
                            if crate::backend::instance::manager::add_instance_item(
                                &inst.path, subfolder, &path,
                            ).is_ok() {
                                count += 1;
                            }
                        }
                        if count > 0 {
                            crate::frontend::toast::show_toast(
                                &self.window,
                                format!("Added {} {} to '{}'", count, label, inst.name),
                            );
                            sender.input(AppMsg::RefreshSelectedInstance);
                        }
                    }
                    EditorOutput::OpenFolder(editor_type) => match editor_type {
                        EditorType::Mods => self.handle_open_mods_folder(),
                        EditorType::ResourcePacks => self.handle_open_resource_packs_folder(),
                        EditorType::ShaderPacks => self.handle_open_shader_packs_folder(),
                        EditorType::Worlds => self.handle_open_worlds_folder(),
                        _ => self.handle_open_instance_folder(),
                    },
                    EditorOutput::BrowseModrinth(editor_type) => {
                        self.handle_browse_modrinth(editor_type);
                    }
                    EditorOutput::RenameWorld(folder, new_name) => {
                        let _ = crate::backend::instance::manager::rename_world(
                            &inst.path, &folder, &new_name,
                        );
                    }
                    EditorOutput::MoveItems(editor_type, ids) => {
                        self.show_target_instance_selector(
                            editor_type,
                            ids,
                            false,
                            sender.input_sender().clone(),
                        );
                    }
                    EditorOutput::CopyItems(editor_type, ids) => {
                        self.show_target_instance_selector(
                            editor_type,
                            ids,
                            true,
                            sender.input_sender().clone(),
                        );
                    }
                    EditorOutput::CheckModsUpdates(items) => {
                        let (loader, _) = inst.get_loader_info();
                        let gv = inst
                            .minecraft_version
                            .clone()
                            .unwrap_or_else(|| "1.20.1".to_string());
                        let sender_clone = sender.input_sender().clone();
                        let loaders = vec![loader.to_string().to_lowercase()];

                        let mut hash_to_filename = std::collections::HashMap::new();
                        let mut hashes = Vec::new();
                        let mods_dir = inst.minecraft_dir.join("mods");
                        for m in items {
                            let mod_path = mods_dir.join(&m.filename);
                            if let Ok(file_bytes) = std::fs::read(&mod_path) {
                                use sha1::{Digest, Sha1};
                                let mut hasher = Sha1::new();
                                hasher.update(&file_bytes);
                                let hash_str = hex::encode(hasher.finalize());
                                hash_to_filename.insert(hash_str.clone(), m.filename.clone());
                                hashes.push(hash_str);
                            }
                        }

                        crate::backend::core::tasks::spawn_io(move || {
                            match crate::backend::download::sources::modrinth::check_updates(
                                hashes,
                                loaders,
                                vec![gv],
                            ) {
                                Ok(result) => {
                                    let mut updates = Vec::new();
                                    for (hash, version) in result {
                                        if let Some(filename) = hash_to_filename.get(&hash) {
                                            let up_to_date = version.files.iter().any(|f| {
                                                f.hashes
                                                    .values()
                                                    .any(|h| h.to_lowercase() == hash.to_lowercase())
                                            });

                                            if !up_to_date {
                                                if let Some(file) = version
                                                    .files
                                                    .iter()
                                                    .find(|f| f.primary)
                                                    .or_else(|| version.files.first())
                                                {
                                                    updates.push(ModUpdateInfo {
                                                        filename: filename.clone(),
                                                        new_version: version.version_number.clone(),
                                                        new_filename: file.filename.clone(),
                                                        version_id: version.id.clone(),
                                                        project_id: version.project_id.clone(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    let _ = sender_clone
                                        .send(AppMsg::ModUpdatesResult(Ok(updates)));
                                }
                                Err(e) => {
                                    let _ = sender_clone.send(AppMsg::ModUpdatesResult(Err(e)));
                                }
                            }
                        });
                    }
                    EditorOutput::UpdateMod(filename, project_id, version_id) => {
                        let gv = inst
                            .minecraft_version
                            .clone()
                            .unwrap_or_else(|| "1.20.1".to_string());
                        let (loader, _) = inst.get_loader_info();
                        let target_dir = inst.minecraft_dir.join("mods");
                        let temp_dir = target_dir
                            .parent()
                            .unwrap()
                            .join(format!("temp_update_{}", uuid::Uuid::new_v4()));
                        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                            eprintln!("Failed to create temp mods dir: {}", e);
                            return;
                        }

                        self.download_status_bar
                            .emit(DownloadStatusBarInput::Update(
                                DownloadState::Starting,
                                true,
                            ));

                        let task = std::sync::Arc::new(
                            crate::backend::download::manager::ModrinthDownloadTask {
                                project_id,
                                version_id: Some(version_id),
                                game_version: gv,
                                loader,
                                mods_dir: temp_dir.clone(),
                                old_filename: Some(filename.clone()),
                            },
                        );

                        let job = crate::backend::download::manager::NetworkJob {
                            id: format!("update-mod-{}", uuid::Uuid::new_v4()),
                            title: format!("Updating mod {}", filename),
                            tasks: vec![task],
                            status: crate::backend::download::manager::NetworkJobStatus::Pending,
                            log: Vec::new(),
                            items: Vec::new(),
                        };

                        let (tx, rx) = std::sync::mpsc::channel::<
                            crate::backend::download::manager::DownloadMsg,
                        >();

                        crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);

                        let sender_clone = sender.input_sender().clone();
                        let filename_clone = filename.clone();
                        let temp_dir_clone = temp_dir.clone();

                        crate::backend::core::tasks::spawn_io(move || {
                            let mut last_error = None;
                            while let Ok(msg) = rx.recv() {
                                if let crate::backend::download::manager::DownloadMsg::Error(err) = msg {
                                    last_error = Some(err);
                                }
                            }

                            let success = temp_dir_clone
                                .join(format!("{}.success", filename_clone))
                                .exists();
                            let _ = std::fs::remove_dir_all(&temp_dir_clone);

                            if success {
                                let _ =
                                    sender_clone.send(AppMsg::ModUpdateSuccess(filename_clone));
                            } else if let Some(err) = last_error {
                                let _ = sender_clone.send(AppMsg::DownloadError(err));
                            }
                            let _ = sender_clone.send(AppMsg::RefreshSelectedInstance);
                        });
                    }
                    EditorOutput::UpdateAllMods(updates) => {
                        let gv = inst
                            .minecraft_version
                            .clone()
                            .unwrap_or_else(|| "1.20.1".to_string());
                        let (loader, _) = inst.get_loader_info();
                        let target_dir = inst.minecraft_dir.join("mods");
                        let temp_dir = target_dir
                            .parent()
                            .unwrap()
                            .join(format!("temp_update_{}", uuid::Uuid::new_v4()));
                        if let Err(e) = std::fs::create_dir_all(&temp_dir) {
                            eprintln!("Failed to create temp mods dir: {}", e);
                            return;
                        }

                        self.download_status_bar
                            .emit(DownloadStatusBarInput::Update(
                                DownloadState::Starting,
                                true,
                            ));

                        let mut tasks: Vec<
                            std::sync::Arc<dyn crate::backend::download::manager::DownloadTask>,
                        > = Vec::new();
                        let mut filenames = Vec::new();

                        for (filename, project_id, version_id) in updates {
                            let task = std::sync::Arc::new(
                                crate::backend::download::manager::ModrinthDownloadTask {
                                    project_id,
                                    version_id: Some(version_id),
                                    game_version: gv.clone(),
                                    loader: loader.clone(),
                                    mods_dir: temp_dir.clone(),
                                    old_filename: Some(filename.clone()),
                                },
                            );
                            tasks.push(task);
                            filenames.push(filename);
                        }

                        let job = crate::backend::download::manager::NetworkJob {
                            id: format!("update-all-mods-{}", uuid::Uuid::new_v4()),
                            title: "Updating all mods".to_string(),
                            tasks,
                            status: crate::backend::download::manager::NetworkJobStatus::Pending,
                            log: Vec::new(),
                            items: Vec::new(),
                        };

                        let (tx, rx) = std::sync::mpsc::channel::<
                            crate::backend::download::manager::DownloadMsg,
                        >();

                        crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);

                        let sender_clone = sender.input_sender().clone();
                        let temp_dir_clone = temp_dir.clone();
                        let filenames_clone = filenames.clone();

                        crate::backend::core::tasks::spawn_io(move || {
                            let mut errors = Vec::new();
                            while let Ok(msg) = rx.recv() {
                                if let crate::backend::download::manager::DownloadMsg::Error(err) = msg {
                                    errors.push(err);
                                }
                            }

                            let mut successful_updates = Vec::new();
                            for filename in &filenames_clone {
                                let marker_path =
                                    temp_dir_clone.join(format!("{}.success", filename));
                                if marker_path.exists() {
                                    successful_updates.push(filename.clone());
                                }
                            }

                            let _ = std::fs::remove_dir_all(&temp_dir_clone);

                            if !successful_updates.is_empty() {
                                let _ = sender_clone
                                    .send(AppMsg::ModUpdateAllSuccess(successful_updates));
                            }

                            if !errors.is_empty() {
                                let combined_err = errors.join("\n");
                                let _ = sender_clone.send(AppMsg::DownloadError(combined_err));
                            }
                            let _ = sender_clone.send(AppMsg::RefreshSelectedInstance);
                        });
                    }
                }
                sender.input(AppMsg::RefreshSelectedInstance);
                self.selected_instance = Some(index);
            }
        }
    }

    pub(crate) fn handle_confirm_move_items(
        &mut self,
        sender: &ComponentSender<AppModel>,
        editor_type: EditorType,
        ids: Vec<String>,
        target_idx: usize,
    ) {
        if let Some(source_idx) = self.selected_instance {
            let source_inst = &self.instances[source_idx];
            let target_inst = &self.instances[target_idx];

            let subfolder = match editor_type {
                EditorType::Mods => "mods",
                EditorType::ResourcePacks => "resourcepacks",
                EditorType::ShaderPacks => "shaderpacks",
                EditorType::Worlds => "saves",
                _ => "mods",
            };

            for id in ids {
                let source_path = source_inst.minecraft_dir.join(subfolder).join(&id);
                if let Err(e) = add_instance_item(&target_inst.path, subfolder, &source_path) {
                    eprintln!("Failed to copy item for move: {}", e);
                } else if let Err(e) = remove_instance_item(&source_inst.path, subfolder, &id) {
                    eprintln!("Failed to remove source item after move: {}", e);
                }
            }
            sender.input(AppMsg::RefreshInstances);
        }
    }

    pub(crate) fn handle_confirm_copy_items(
        &mut self,
        sender: &ComponentSender<AppModel>,
        editor_type: EditorType,
        ids: Vec<String>,
        target_idx: usize,
    ) {
        if let Some(source_idx) = self.selected_instance {
            let source_inst = &self.instances[source_idx];
            let target_inst = &self.instances[target_idx];

            let subfolder = match editor_type {
                EditorType::Mods => "mods",
                EditorType::ResourcePacks => "resourcepacks",
                EditorType::ShaderPacks => "shaderpacks",
                EditorType::Worlds => "saves",
                _ => "mods",
            };

            for id in ids {
                let source_path = source_inst.minecraft_dir.join(subfolder).join(&id);
                if let Err(e) = add_instance_item(&target_inst.path, subfolder, &source_path) {
                    eprintln!("Failed to copy item: {}", e);
                }
            }
            sender.input(AppMsg::RefreshInstances);
        }
    }
}
