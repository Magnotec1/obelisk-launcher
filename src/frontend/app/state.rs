use super::msg::{AppMsg, InstanceStatus};
use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::Instance;
use crate::backend::playtime::PlaytimeManager;
use crate::config::Config;
use crate::frontend::dialogs::external::browser::UnifiedBrowser;
use crate::frontend::dialogs::external::download::{DownloadDialog, DownloadStatusBar};
use crate::frontend::dialogs::instance::add::AddInstanceDialog;
use crate::frontend::dialogs::instance::components::ComponentEditorDialog;
use crate::frontend::dialogs::instance::editor::{EditorType, InstanceEditorDialog};
use crate::frontend::dialogs::instance::mod_loader::ModLoaderDialog;
use crate::frontend::dialogs::instance::sharing::{ImportDialog, InstanceSharerDialog};
use crate::frontend::dialogs::system::java::JavaSelectorDialog;
use crate::frontend::dialogs::system::setup::SetupDialog;
use crate::frontend::dialogs::system::shortcuts::ShortcutsDialog;
use crate::frontend::views::account::AccountView;
use crate::frontend::views::assets::AssetManagerView;
use crate::frontend::views::discover::DiscoverView;
use crate::frontend::views::instance::{
    ConsoleOutput, EditorTabOutput, InstanceConsole, InstanceEditorTab, InstanceSettingsTab,
    InstanceSummary, SettingsTabOutput, SummaryOutput,
};
use crate::frontend::views::library::{LayoutMode, OverviewGrid};
use crate::frontend::views::playtime::PlaytimeView;
use crate::frontend::views::settings::SettingsDialog;
use crate::frontend::views::sidebar::{InstanceSidebar, SidebarList, SidebarPage};
use adw::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};

pub struct AppModel {
    pub(crate) config: Config,
    pub(crate) instances: Vec<Instance>,
    pub(crate) groups: InstanceGroups,
    pub(crate) selected_instance: Option<usize>,
    pub(crate) add_instance_dialog: Controller<AddInstanceDialog>,
    pub(crate) instance_editor: Controller<InstanceEditorDialog>,
    pub(crate) download_dialog: Controller<DownloadDialog>,
    pub(crate) java_selector: Controller<JavaSelectorDialog>,
    pub(crate) component_editor: Controller<ComponentEditorDialog>,
    pub(crate) mod_loader_dialog: Controller<ModLoaderDialog>,
    pub(crate) browser_dialog: Controller<UnifiedBrowser>,

    pub(crate) sharer_dialog: Controller<InstanceSharerDialog>,
    pub(crate) import_dialog: Controller<ImportDialog>,
    pub(crate) playtime_view: Controller<PlaytimeView>,
    pub(crate) shortcuts_dialog: Controller<ShortcutsDialog>,

    // Sidebar + overview
    pub(crate) sidebar: Controller<SidebarList>,
    pub(crate) instance_sidebar: Controller<InstanceSidebar>,
    pub(crate) overview_grid: Controller<OverviewGrid>,
    pub(crate) active_sidebar_page: SidebarPage,

    // Views
    pub(crate) instance_summary: Controller<InstanceSummary>,
    pub(crate) instance_editor_tab: Controller<InstanceEditorTab>,
    pub(crate) instance_console: Controller<InstanceConsole>,
    pub(crate) instance_settings_tab: Controller<InstanceSettingsTab>,
    pub(crate) account_view: Controller<AccountView>,
    pub(crate) asset_view: Controller<AssetManagerView>,
    pub(crate) discover_view: Controller<DiscoverView>,
    pub(crate) settings_dialog: Controller<SettingsDialog>,
    pub(crate) setup_dialog: Controller<SetupDialog>,
    pub(crate) download_status_bar: Controller<DownloadStatusBar>,

    pub(crate) window: adw::Window,
    pub(crate) split_view: adw::OverlaySplitView,

    // Auth state
    pub(crate) auth_in_progress: bool,

    // Launch state
    pub(crate) instance_statuses: HashMap<PathBuf, InstanceStatus>,
    pub(crate) instance_processes: HashMap<PathBuf, Arc<Mutex<Option<Child>>>>,
    pub(crate) instance_consoles: HashMap<PathBuf, gtk::TextBuffer>,
    pub(crate) instance_logs: HashMap<PathBuf, Vec<String>>,
    pub(crate) default_console_buffer: gtk::TextBuffer,
    pub(crate) active_tab: String,
    pub(crate) console_search_query: String,

    // Download / UI state
    pub(crate) loading_instances: bool,
    pub(crate) launch_after_download: bool,
    pub(crate) toast_overlay: adw::ToastOverlay,
    pub(crate) active_editor_type: Option<EditorType>,
    pub(crate) is_narrow: bool,
    pub(crate) overview_layout: LayoutMode,
    pub(crate) current_folder: Option<String>,
    pub(crate) playtime_manager: PlaytimeManager,
    pub(crate) sharing_loading: bool,
    pub(crate) import_loading: bool,
    pub(crate) verifying_loading: bool,
    pub(crate) installing_modpack: bool,
    pub(crate) active_asset_subpage: Option<String>,
    pub(crate) discover_search_visible: bool,
    pub(crate) discover_search_query: String,
    pub(crate) discover_details_open: bool,
    pub(crate) discover_details_title: String,
}

impl AppModel {
    pub(crate) fn get_active_console_buffer(&self) -> gtk::TextBuffer {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                if let Some(buf) = self.instance_consoles.get(&inst.path) {
                    return buf.clone();
                }
            }
        }
        self.default_console_buffer.clone()
    }

    pub(crate) fn get_active_instance_path(&self) -> Option<PathBuf> {
        self.selected_instance
            .and_then(|i| self.instances.get(i))
            .map(|inst| inst.path.clone())
    }

    pub(crate) fn get_console_buffer(&mut self, path: &std::path::Path) -> gtk::TextBuffer {
        self.instance_logs.entry(path.to_path_buf()).or_default();
        self.instance_consoles
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let buf = gtk::TextBuffer::new(None);
                buf.set_text(&format!("Console initialized for {}\n", path.display()));
                buf
            })
            .clone()
    }

    pub(crate) fn rebuild_console_buffer(&mut self, path: &PathBuf) {
        let buf = self.get_console_buffer(path);
        buf.set_text("");

        if let Some(logs) = self.instance_logs.get(path) {
            let mut iter = buf.end_iter();
            let query = self.console_search_query.to_lowercase();
            for line in logs {
                if query.is_empty() || line.to_lowercase().contains(&query) {
                    buf.insert(&mut iter, line);
                }
            }
        }
    }

    pub(crate) fn get_instance_status(&self, path: &std::path::Path) -> InstanceStatus {
        self.instance_statuses.get(path).copied().unwrap_or(InstanceStatus::NotRunning)
    }

    pub(crate) fn get_active_instance_status(&self) -> InstanceStatus {
        if let Some(path) = self.get_active_instance_path() {
            return self.get_instance_status(&path);
        }
        InstanceStatus::NotRunning
    }

    pub(crate) fn get_active_instance_has_logs(&self) -> bool {
        if let Some(path) = self.get_active_instance_path() {
            if let Some(logs) = self.instance_logs.get(&path) {
                return !logs.is_empty();
            }
        }
        false
    }

    pub(crate) fn get_game_process(&mut self, path: &std::path::Path) -> Arc<Mutex<Option<Child>>> {
        self.instance_processes
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    pub(crate) fn handle_summary_output(&mut self, output: SummaryOutput, sender: ComponentSender<Self>) {
        match output {
            SummaryOutput::Launch => sender.input(AppMsg::LaunchInstance),
            SummaryOutput::Verify => sender.input(AppMsg::VerifyInstance),
            SummaryOutput::Kill => sender.input(AppMsg::KillInstance),
            SummaryOutput::OpenFolder => sender.input(AppMsg::OpenInstanceFolder),
            SummaryOutput::SwitchToConsole => {
                sender.input(AppMsg::SwitchTab("console".to_string()))
            }
            SummaryOutput::Share => {
                if let Some(idx) = self.selected_instance {
                    sender.input(AppMsg::ShareInstance(idx));
                }
            }
        }
    }

    pub(crate) fn handle_editor_output(&mut self, output: EditorTabOutput, sender: ComponentSender<Self>) {
        match output {
            EditorTabOutput::EditMods => sender.input(AppMsg::EditMods),
            EditorTabOutput::ExploreMods => sender.input(AppMsg::BrowseModrinth(EditorType::Mods)),
            EditorTabOutput::EditComponents => sender.input(AppMsg::EditComponents),
            EditorTabOutput::EditResourcePacks => sender.input(AppMsg::EditResourcePacks),
            EditorTabOutput::ExploreResourcePacks => sender.input(AppMsg::BrowseModrinth(EditorType::ResourcePacks)),
            EditorTabOutput::EditShaderPacks => sender.input(AppMsg::EditShaderPacks),
            EditorTabOutput::ExploreShaderPacks => sender.input(AppMsg::BrowseModrinth(EditorType::ShaderPacks)),
            EditorTabOutput::EditWorlds => sender.input(AppMsg::EditWorlds),
            EditorTabOutput::OpenScreenshotsFolder => sender.input(AppMsg::OpenScreenshotsFolder),
            EditorTabOutput::OpenJavaSelector => sender.input(AppMsg::OpenJavaSelector),
            EditorTabOutput::SetInstanceJavaDefault => sender.input(AppMsg::SetInstanceJavaDefault),
            EditorTabOutput::OpenComponentSwap(uid) => sender.input(AppMsg::OpenComponentSwap(uid)),
            EditorTabOutput::RemoveComponent(uid) => sender.input(AppMsg::RemoveComponent(uid)),
            EditorTabOutput::SelectModLoaderRequest => sender.input(AppMsg::SelectModLoaderRequest),
        }
    }

    pub(crate) fn handle_settings_output(&mut self, output: SettingsTabOutput, sender: ComponentSender<Self>) {
        match output {
            SettingsTabOutput::SetFeralGameMode(e) => {
                sender.input(AppMsg::SetInstanceFeralGameMode(e))
            }
            SettingsTabOutput::SetDiscreteGpu(e) => sender.input(AppMsg::SetInstanceDiscreteGpu(e)),
            SettingsTabOutput::SetZinkVulkan(e) => sender.input(AppMsg::SetInstanceZinkVulkan(e)),
            SettingsTabOutput::SetUseWayland(e) => sender.input(AppMsg::SetInstanceUseWayland(e)),
        }
    }

    pub(crate) fn handle_console_output(&mut self, output: ConsoleOutput, sender: ComponentSender<Self>) {
        match output {
            ConsoleOutput::Launch => sender.input(AppMsg::LaunchInstance),
            ConsoleOutput::Kill => sender.input(AppMsg::KillInstance),
            ConsoleOutput::Search(query) => sender.input(AppMsg::SetConsoleSearchQuery(query)),
        }
    }

    pub(crate) fn has_selected_mismatch(&self) -> bool {
        if let Some(idx) = self.selected_instance {
            if let Some(inst) = self.instances.get(idx) {
                return inst.has_mismatch;
            }
        }
        false
    }

    pub(crate) fn rebuild_overview(&self) {
        let mut statuses = HashMap::new();
        for inst in &self.instances {
            let status = self.get_instance_status(&inst.path);
            statuses.insert(inst.path.clone(), status);
        }
        self.overview_grid.emit(crate::frontend::views::library::OverviewInput::Rebuild(
            self.instances.clone(),
            self.groups.clone(),
            statuses,
        ));
    }

    pub(crate) fn show_move_to_group_dialog(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        let dialog_win = adw::Dialog::builder()
            .title("Move to Group")
            .content_width(380)
            .can_close(true)
            .build();

        let content = adw::ToolbarView::builder().build();
        let header = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .build();
        let window_title = adw::WindowTitle::builder().title("Move to Group").build();
        header.set_title_widget(Some(&window_title));
        content.add_top_bar(&header);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(200)
            .max_content_height(350)
            .build();

        let lb = gtk::ListBox::builder()
            .css_classes(vec!["boxed-list".to_string()])
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .valign(gtk::Align::Start)
            .build();

        let cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .css_classes(vec!["pill".to_string()])
            .halign(gtk::Align::Center)
            .width_request(120)
            .build();
        {
            let dw = dialog_win.clone();
            cancel_btn.connect_clicked(move |_| {
                dw.close();
            });
        }
        content.add_bottom_bar(&cancel_btn);

        let group_names = self.groups.sorted_group_names();
        for gname in group_names {
            let row = adw::ActionRow::builder()
                .title(gname)
                .activatable(true)
                .build();
            let icon = gtk::Image::from_icon_name("folder-symbolic");
            row.add_prefix(&icon);
            let dw = dialog_win.clone();
            let s = sender.input_sender().clone();
            let gn = gname.to_string();
            row.connect_activated(move |_| {
                dw.close();
                let _ = s.send(AppMsg::MoveInstanceToGroup(idx, gn.clone()));
            });
            lb.append(&row);
        }

        let new_group_row = adw::ActionRow::builder()
            .title("New Group…")
            .activatable(true)
            .build();
        let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
        new_group_row.add_prefix(&add_icon);
        {
            let dw = dialog_win.clone();
            let s = sender.input_sender().clone();
            new_group_row.connect_activated(move |_| {
                dw.close();
                let _ = s.send(AppMsg::CreateGroupWithMove(idx));
            });
        }
        lb.append(&new_group_row);

        scrolled.set_child(Some(&lb));
        content.set_content(Some(&scrolled));
        dialog_win.set_child(Some(&content));
        dialog_win.present(Some(&self.window));
    }

    pub(crate) fn show_create_group_dialog_then_move(
        &self,
        sender: &ComponentSender<AppModel>,
        move_idx: Option<usize>,
    ) {
        let dialog = adw::AlertDialog::builder()
            .heading("Create Group")
            .body("Enter a name for the new group:")
            .close_response("cancel")
            .default_response("create")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("create", "Create");
        dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

        let entry = gtk::Entry::builder()
            .placeholder_text("Group name...")
            .activates_default(true)
            .build();
        dialog.set_extra_child(Some(&entry));

        let sender_clone = sender.input_sender().clone();
        dialog.choose(
            &self.window,
            None::<&gtk::gio::Cancellable>,
            move |response| {
                if response == "create" {
                    let name = entry.text().to_string();
                    sender_clone
                        .send(AppMsg::ConfirmCreateGroup(name.clone()))
                        .unwrap();
                    if let Some(idx) = move_idx {
                        sender_clone
                            .send(AppMsg::MoveInstanceToGroup(idx, name))
                            .unwrap();
                    }
                }
            },
        );
    }

    pub(crate) fn show_target_instance_selector(
        &self,
        editor_type: EditorType,
        ids: Vec<String>,
        is_copy: bool,
        sender_clone: relm4::Sender<AppMsg>,
    ) {
        if let Some(current_inst_index) = self.selected_instance {
            let instances = self.instances.clone();
            let ids_clone = ids.clone();
            let type_clone = editor_type.clone();

            let heading = match (is_copy, ids.len()) {
                (true, 1) => "Copy Item".to_string(),
                (true, _) => format!("Copy {} Items", ids.len()),
                (false, 1) => "Move Item".to_string(),
                (false, _) => format!("Move {} Items", ids.len()),
            };

            let dialog = adw::Dialog::builder()
                .title(&heading)
                .content_width(360)
                .content_height(380)
                .build();

            let list_box = gtk::ListBox::new();
            list_box.set_selection_mode(gtk::SelectionMode::Single);
            list_box.add_css_class("boxed-list");

            let mut inst_indices = Vec::new();
            for (idx, other) in instances.iter().enumerate() {
                if idx == current_inst_index {
                    continue;
                }
                inst_indices.push(idx);
                let row = adw::ActionRow::builder()
                    .title(&other.name)
                    .activatable(true)
                    .build();
                list_box.append(&row);
            }

            let scrolled = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .min_content_height(200)
                .max_content_height(300)
                .child(&list_box)
                .build();

            let content_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(12)
                .margin_start(16)
                .margin_end(16)
                .margin_top(16)
                .margin_bottom(16)
                .build();

            let label = gtk::Label::builder()
                .label("Select the target instance:")
                .halign(gtk::Align::Start)
                .build();
            content_box.append(&label);
            content_box.append(&scrolled);

            let cancel_btn = gtk::Button::builder()
                .label("Cancel")
                .css_classes(vec!["flat"])
                .halign(gtk::Align::End)
                .build();

            {
                let dialog_clone = dialog.clone();
                cancel_btn.connect_clicked(move |_| {
                    dialog_clone.close();
                });
            }
            content_box.append(&cancel_btn);

            dialog.set_child(Some(&content_box));

            let dialog_clone = dialog.clone();
            list_box.connect_row_activated(move |_lb, row| {
                let selected_row_idx = row.index() as usize;
                if let Some(&target_idx) = inst_indices.get(selected_row_idx) {
                    if is_copy {
                        sender_clone
                            .send(AppMsg::ConfirmCopyItems(
                                type_clone.clone(),
                                ids_clone.clone(),
                                target_idx,
                            ))
                            .ok();
                    } else {
                        sender_clone
                            .send(AppMsg::ConfirmMoveItems(
                                type_clone.clone(),
                                ids_clone.clone(),
                                target_idx,
                            ))
                            .ok();
                    }
                    dialog_clone.close();
                }
            });

            dialog.present(Some(&self.window));
        }
    }
}
