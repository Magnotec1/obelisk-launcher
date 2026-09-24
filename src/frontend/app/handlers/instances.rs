use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::{
    delete_instance, rename_instance, scan_instances, scan_single_instance, Instance,
};
use crate::backend::runtime::versions::MinecraftVersion;
use crate::frontend::app::msg::{AppMsg, InstanceStatus};
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::instance::add::AddInstanceInput;
use crate::frontend::dialogs::instance::editor::{EditorInput, EditorItem, EditorType};
use crate::frontend::views::instance::{
    ConsoleInput, EditorTabInput, SettingsTabInput, SummaryInput,
};
use crate::frontend::views::library::OverviewInput;
use crate::frontend::views::sidebar::{SidebarInput, SidebarPage};
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

impl AppModel {
    pub(crate) fn handle_select_instance(&mut self, sender: &ComponentSender<AppModel>, index: usize) {
        self.selected_instance = Some(index);
        self.active_sidebar_page = SidebarPage::InstanceDetails;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Library));
        if self.split_view.is_collapsed() {
            self.split_view.set_show_sidebar(false);
        }
        self.active_tab = "summary".to_string();
        let inst_opt = self.instances.get(index).cloned();
        let status = self.get_active_instance_status();

        self.instance_summary
            .emit(SummaryInput::Update(Box::new(inst_opt.clone()), status));
        self.instance_summary
            .emit(SummaryInput::SetSharingLoading(self.sharing_loading));
        self.instance_summary
            .emit(SummaryInput::SetVerifyingLoading(self.verifying_loading));
        self.instance_editor_tab.emit(EditorTabInput::Update(
            inst_opt.clone(),
            self.config.clone(),
        ));
        self.instance_settings_tab.emit(SettingsTabInput::Update(
            Box::new(inst_opt.clone()),
            Box::new(self.config.clone()),
        ));
        self.instance_console.emit(ConsoleInput::Update {
            buffer: self.get_active_console_buffer(),
            status,
            has_any_logs: self.get_active_instance_has_logs(),
        });

        // Trigger full scan in background for selected instance so all mod metadata, versions, icons, and worlds load immediately
        self.handle_refresh_selected_instance(sender);
    }

    pub(crate) fn handle_add_instance(&mut self, group: Option<String>) {
        let available_groups = self
            .groups
            .sorted_group_names()
            .into_iter()
            .map(String::from)
            .collect();
        self.add_instance_dialog.emit(AddInstanceInput::Open {
            target_group: group,
            available_groups,
        });
        self.add_instance_dialog
            .widget()
            .present(Some(&self.window));
    }

    pub(crate) fn handle_header_add_instance(&mut self, sender: &ComponentSender<AppModel>) {
        sender.input(AppMsg::AddInstance(self.current_folder.clone()));
    }

    pub(crate) fn handle_instance_created(
        &mut self,
        sender: &ComponentSender<AppModel>,
        _version: MinecraftVersion,
        _path: PathBuf,
        _group: Option<String>,
    ) {
        sender.input(AppMsg::RefreshInstances);
    }

    pub(crate) fn handle_refresh_instances(&mut self, sender: &ComponentSender<AppModel>) {
        self.loading_instances = true;
        self.overview_grid.emit(OverviewInput::SetLoading(true));
        self.active_sidebar_page = SidebarPage::Library;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Library));
        if let Some(path) = &self.config.instances_path {
            let path_clone = path.clone();
            let sender_clone = sender.input_sender().clone();
            crate::backend::core::tasks::spawn_io(move || {
                let insts = scan_instances(&path_clone);
                let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
            });
        }
    }

    pub(crate) fn handle_refresh_selected_instance(&self, sender: &ComponentSender<AppModel>) {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                let path = inst.path.clone();
                let sender_clone = sender.input_sender().clone();
                crate::backend::core::tasks::spawn_io(move || {
                    if let Some(updated) = scan_single_instance(&path, true) {
                        let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
                    }
                });
            }
        }
    }

    pub(crate) fn handle_selected_instance_updated(&mut self, updated_inst: Instance) {
        if let Some(index) = self.selected_instance {
            if let Some(existing) = self.instances.get_mut(index) {
                if existing.path == updated_inst.path {
                    *existing = updated_inst.clone();

                    let status = self.get_active_instance_status();
                    self.instance_summary
                        .emit(SummaryInput::Update(Box::new(Some(updated_inst.clone())), status));
                    self.instance_editor_tab.emit(EditorTabInput::Update(
                        Some(updated_inst.clone()),
                        self.config.clone(),
                    ));
                    self.instance_settings_tab.emit(SettingsTabInput::Update(
                        Box::new(Some(updated_inst.clone())),
                        Box::new(self.config.clone()),
                    ));

                    if let Some(active_type) = &self.active_editor_type {
                        let items = match active_type {
                            EditorType::Mods => updated_inst
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
                                .collect(),
                            EditorType::Components => updated_inst
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
                                .collect(),
                            EditorType::ResourcePacks => updated_inst
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
                                .collect(),
                            EditorType::ShaderPacks => updated_inst
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
                                .collect(),
                            EditorType::Worlds => updated_inst
                                .worlds
                                .iter()
                                .map(|w| {
                                    let size_str =
                                        crate::frontend::utils::format_size(w.file_size);
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
                                .collect(),
                        };
                        self.instance_editor.emit(EditorInput::UpdateItems(items));
                    }
                }
            }
        }
        for (i, inst) in self.instances.iter_mut().enumerate() {
            if inst.path == updated_inst.path {
                *inst = updated_inst.clone();
                if self.selected_instance == Some(i) {
                    let status = self.get_active_instance_status();
                    self.instance_summary
                        .emit(SummaryInput::Update(Box::new(Some(updated_inst.clone())), status));
                    self.instance_editor_tab.emit(EditorTabInput::Update(
                        Some(updated_inst.clone()),
                        self.config.clone(),
                    ));
                    self.instance_settings_tab.emit(SettingsTabInput::Update(
                        Box::new(Some(updated_inst)),
                        Box::new(self.config.clone()),
                    ));
                }
                break;
            }
        }
        self.rebuild_overview();
    }

    pub(crate) fn handle_instances_updated(&mut self, sender: &ComponentSender<AppModel>, instances: Vec<Instance>) {
        self.loading_instances = false;
        self.overview_grid.emit(OverviewInput::SetLoading(false));
        let old_selection = self.selected_instance;
        self.instances = instances;
        self.playtime_manager.ensure_initialized(&self.instances);

        if let Some(path) = &self.config.instances_path {
            self.groups = InstanceGroups::load(path);
        }

        self.rebuild_overview();

        if let Some(idx) = old_selection {
            if idx < self.instances.len() {
                sender.input(AppMsg::SelectInstance(idx));
            } else {
                self.selected_instance = None;
                self.active_sidebar_page = SidebarPage::Library;
                self.sidebar
                    .emit(SidebarInput::SetSelected(SidebarPage::Library));
            }
        } else {
            self.selected_instance = None;
        }

        if let Some(index) = self.selected_instance {
            let inst = self.instances.get(index).cloned();
            let status = self.get_active_instance_status();
            self.instance_summary
                .emit(SummaryInput::Update(Box::new(inst.clone()), status));
            self.instance_editor_tab
                .emit(EditorTabInput::Update(inst.clone(), self.config.clone()));
            self.instance_settings_tab
                .emit(SettingsTabInput::Update(Box::new(inst), Box::new(self.config.clone())));
            self.instance_console.emit(ConsoleInput::Update {
                buffer: self.get_active_console_buffer(),
                status,
                has_any_logs: self.get_active_instance_has_logs(),
            });
        }
    }

    pub(crate) fn handle_rename_instance_request(&self, sender: &ComponentSender<AppModel>, index: usize) {
        if let Some(inst) = self.instances.get(index) {
            let current_name = inst.name.clone();
            let dialog = adw::AlertDialog::builder()
                .heading("Rename Instance")
                .body("Enter a new name for the instance:")
                .close_response("cancel")
                .default_response("rename")
                .build();
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("rename", "Rename");
            dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

            let entry = gtk::Entry::builder()
                .text(&current_name)
                .activates_default(true)
                .build();
            dialog.set_extra_child(Some(&entry));

            let sender_clone = sender.input_sender().clone();
            dialog.choose(
                &self.window,
                None::<&gtk::gio::Cancellable>,
                move |response| {
                    if response == "rename" {
                        let new_name = entry.text().to_string();
                        sender_clone
                            .send(AppMsg::ConfirmRename(index, new_name))
                            .ok();
                    }
                },
            );
        }
    }

    pub(crate) fn handle_confirm_rename(&mut self, index: usize, new_name: String) {
        let new_name = new_name.trim().to_string();
        if !new_name.is_empty() {
            if let Some(inst) = self.instances.get_mut(index) {
                let _ = rename_instance(&inst.path, &new_name);
                inst.name = new_name;
                self.instances.sort_by(|a, b| a.name.cmp(&b.name));
                self.rebuild_overview();
            }
        }
    }

    pub(crate) fn handle_delete_instance_request(&self, sender: &ComponentSender<AppModel>, index: usize) {
        if let Some(inst) = self.instances.get(index) {
            let name = inst.name.clone();
            let dialog = adw::AlertDialog::builder()
                .heading("Delete Instance?")
                .body(format!("This will permanently remove '{}' and all its files. This cannot be undone.", name))
                .close_response("cancel")
                .default_response("cancel")
                .build();
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("delete", "Delete");
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

            let sender_clone = sender.input_sender().clone();
            dialog.choose(
                &self.window,
                None::<&gtk::gio::Cancellable>,
                move |response| {
                    if response == "delete" {
                        sender_clone.send(AppMsg::ConfirmDelete(index)).ok();
                    }
                },
            );
        }
    }

    pub(crate) fn handle_confirm_delete(&mut self, index: usize) {
        if let Some(inst) = self.instances.get(index) {
            let folder = inst
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let _ = delete_instance(&inst.path);
            if let Some(path) = &self.config.instances_path {
                self.groups.remove_instance_from_groups(&folder);
                let _ = self.groups.save(path);
            }
        }
        if index < self.instances.len() {
            self.instances.remove(index);
        }
        self.selected_instance = None;
        self.active_sidebar_page = SidebarPage::Library;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Library));
        self.rebuild_overview();
    }

    pub(crate) fn handle_change_instance_icon_from_file(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        if let Some(_inst) = self.instances.get(idx) {
            let s = sender.input_sender().clone();

            let dialog = gtk::FileDialog::builder()
                .title("Select Instance Icon")
                .accept_label("Select")
                .modal(true)
                .build();

            let filters = gtk::FileFilter::new();
            filters.add_mime_type("image/png");
            filters.add_mime_type("image/jpeg");
            filters.set_name(Some("Images"));
            let list_store = gtk::gio::ListStore::new::<gtk::FileFilter>();
            list_store.append(&filters);
            dialog.set_filters(Some(&list_store));

            dialog.open(
                Some(&self.window),
                None::<&gtk::gio::Cancellable>,
                move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            let _ = s.send(AppMsg::ApplyIconPath(idx, path));
                        }
                    }
                },
            );
        }
    }

    pub(crate) fn handle_apply_default_icon(&mut self, sender: &ComponentSender<AppModel>, idx: usize) {
        if let Some(default_path) = self.config.default_instance_icon.clone() {
            if default_path.exists() {
                self.handle_apply_icon_path(sender, idx, default_path);
                return;
            }
        }

        if let Some(inst) = self.instances.get(idx) {
            let inst_path = inst.path.clone();
            let mc_dir = inst.minecraft_dir.clone();
            let target1 = inst_path.join("icon.png");
            let target2 = mc_dir.join("icon.png");

            let _ = std::fs::remove_file(&target1);
            let _ = std::fs::remove_file(&target2);

            self.overview_grid
                .emit(OverviewInput::ClearTextureCache(target1));
            self.overview_grid
                .emit(OverviewInput::ClearTextureCache(target2));

            if self.config.is_demo {
                if let Some(selected_idx) = self.selected_instance {
                    if selected_idx == idx {
                        if let Some(current) = self.instances.get(idx) {
                            self.instance_summary.emit(SummaryInput::Update(
                                Box::new(Some(current.clone())),
                                InstanceStatus::NotRunning,
                            ));
                        }
                    }
                }
                self.rebuild_overview();
                return;
            }

            let _ = crate::backend::instance::manager::update_cfg_key(
                &inst_path, "iconKey", "default",
            );
            let sender_clone = sender.input_sender().clone();
            crate::backend::core::tasks::spawn_io(move || {
                if let Some(updated) = scan_single_instance(&inst_path, true) {
                    let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
                    let _ = sender_clone.send(AppMsg::RefreshInstances);
                }
            });
        }
    }

    pub(crate) fn handle_apply_icon_path(&mut self, sender: &ComponentSender<AppModel>, idx: usize, source_path: PathBuf) {
        if let Some(inst) = self.instances.get(idx) {
            let inst_path = inst.path.clone();
            let target = inst_path.join("icon.png");
            if std::fs::copy(&source_path, &target).is_ok() {
                self.overview_grid
                    .emit(OverviewInput::ClearTextureCache(target));
                if self.config.is_demo {
                    if let Some(selected_idx) = self.selected_instance {
                        if selected_idx == idx {
                            if let Some(current) = self.instances.get(idx) {
                                self.instance_summary.emit(SummaryInput::Update(
                                    Box::new(Some(current.clone())),
                                    InstanceStatus::NotRunning,
                                ));
                            }
                        }
                    }
                    self.rebuild_overview();
                    return;
                }
                let _ = crate::backend::instance::manager::update_cfg_key(
                    &inst_path, "iconKey", "custom",
                );
                let sender_clone = sender.input_sender().clone();
                crate::backend::core::tasks::spawn_io(move || {
                    if let Some(updated) = scan_single_instance(&inst_path, true) {
                        let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
                        let _ = sender_clone.send(AppMsg::RefreshInstances);
                    }
                });
            }
        }
    }

    pub(crate) fn handle_open_instance_folder(&self) {
        if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
            crate::frontend::utils::file::open_instance_subfolder(&inst.path, "");
        }
    }

    pub(crate) fn handle_create_group_request(&self, sender: &ComponentSender<AppModel>) {
        self.show_create_group_dialog_then_move(sender, None);
    }

    pub(crate) fn handle_confirm_create_group(&mut self, name: String) {
        let name = name.trim().to_string();
        if !name.is_empty() {
            if let Some(path) = &self.config.instances_path {
                self.groups.create_group(&name);
                let _ = self.groups.save(path);
            }
            self.rebuild_overview();
        }
    }

    pub(crate) fn handle_move_to_group_request(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        self.show_move_to_group_dialog(sender, idx);
    }

    pub(crate) fn handle_create_group_with_move(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        self.show_create_group_dialog_then_move(sender, Some(idx));
    }

    pub(crate) fn handle_move_instance_to_group(&mut self, idx: usize, group: String) {
        if let Some(inst) = self.instances.get(idx) {
            let folder = inst
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(path) = &self.config.instances_path {
                self.groups.set_instance_group(&folder, &group);
                let _ = self.groups.save(path);
            }
            self.rebuild_overview();
        }
    }

    pub(crate) fn handle_remove_instance_from_group(&mut self, idx: usize) {
        if let Some(inst) = self.instances.get(idx) {
            let folder = inst
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(path) = &self.config.instances_path {
                self.groups.remove_instance_from_groups(&folder);
                let _ = self.groups.save(path);
            }
            self.rebuild_overview();
        }
    }

    pub(crate) fn handle_rename_group_request(&self, sender: &ComponentSender<AppModel>, old_name: String) {
        let dialog = adw::AlertDialog::builder()
            .heading("Rename Group")
            .body("Enter a new name for the group:")
            .close_response("cancel")
            .default_response("rename")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("rename", "Rename");
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        let entry = gtk::Entry::builder()
            .text(&old_name)
            .activates_default(true)
            .build();
        dialog.set_extra_child(Some(&entry));
        let sender_clone = sender.input_sender().clone();
        let old = old_name.clone();
        dialog.choose(
            &self.window,
            None::<&gtk::gio::Cancellable>,
            move |response| {
                if response == "rename" {
                    let new_name = entry.text().to_string();
                    sender_clone
                        .send(AppMsg::ConfirmRenameGroup(old.clone(), new_name))
                        .ok();
                }
            },
        );
    }

    pub(crate) fn handle_confirm_rename_group(&mut self, old_name: String, new_name: String) {
        let new_name = new_name.trim().to_string();
        if !new_name.is_empty() && new_name != old_name {
            if let Some(path) = &self.config.instances_path {
                self.groups.rename_group(&old_name, &new_name);
                let _ = self.groups.save(path);
            }
            self.rebuild_overview();
        }
    }

    pub(crate) fn handle_delete_group_request(&self, sender: &ComponentSender<AppModel>, name: String) {
        let dialog = adw::AlertDialog::builder()
            .heading("Delete Group?")
            .body(format!("Are you sure you want to delete the group '{}'? Instances in this group will not be deleted.", name))
            .close_response("cancel")
            .default_response("cancel")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("delete", "Delete");
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        let sender_clone = sender.input_sender().clone();
        let group_name = name.clone();
        dialog.choose(
            &self.window,
            None::<&gtk::gio::Cancellable>,
            move |response| {
                if response == "delete" {
                    sender_clone
                        .send(AppMsg::ConfirmDeleteGroup(group_name.clone()))
                        .ok();
                }
            },
        );
    }

    pub(crate) fn handle_confirm_delete_group(&mut self, name: String) {
        if let Some(path) = &self.config.instances_path {
            self.groups.delete_group(&name);
            let _ = self.groups.save(path);
        }
        self.rebuild_overview();
    }
}
