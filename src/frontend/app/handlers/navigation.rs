use crate::config::{PreferredViewType, SortBy};
use crate::frontend::app::msg::AppMsg;
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::system::setup::SetupInput;
use crate::frontend::views::assets::AssetInput;
use crate::frontend::views::instance::SummaryInput;
use crate::frontend::views::library::{LayoutMode, OverviewInput, OverviewOutput};
use crate::frontend::views::playtime::PlaytimeInput;
use crate::frontend::views::settings::SettingsInput;
use crate::frontend::views::sidebar::{SidebarInput, SidebarOutput, SidebarPage};
use adw::prelude::*;
use relm4::prelude::*;

impl AppModel {
    pub(crate) fn handle_open_settings(&mut self) {
        self.settings_dialog
            .emit(SettingsInput::UpdateConfig(self.config.clone()));
        self.settings_dialog
            .emit(SettingsInput::SetPage("general".to_string()));
        self.settings_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_open_setup(&mut self) {
        self.setup_dialog.emit(SetupInput::Open);
        self.setup_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_open_about(&self) {
        let about = adw::AboutDialog::builder()
            .application_name("Obelisk Launcher")
            .version(env!("CARGO_PKG_VERSION"))
            .developer_name("Magnotec")
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/Magnotec1/obelisk-launcher")
            .issue_url("https://github.com/Magnotec1/obelisk-launcher/issues")
            .comments(
                "A modern Minecraft instance manager built with Rust and GTK4/Libadwaita. Designed around the same format as MultiMC/PolyMC/Prism Launcher, for compatibility.",
            )
            .build();
        about.present(Some(&self.window));
    }

    pub(crate) fn handle_open_shortcuts(&self) {
        self.shortcuts_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_open_asset_manager(&mut self, sender: &ComponentSender<AppModel>) {
        self.active_sidebar_page = SidebarPage::Assets;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Assets));
        self.active_asset_subpage = None;
        sender.input(AppMsg::RefreshAssets);
    }

    pub(crate) fn handle_refresh_assets(&self, sender: &ComponentSender<AppModel>) {
        let data_path = self.config.minecraft_data_path.clone();
        let shared_path = self.config.shared_data_path.clone();
        let instances_path = self.config.instances_path.clone();
        self.asset_view.emit(AssetInput::Loading(true));

        let sender_clone = sender.input_sender().clone();
        crate::backend::core::tasks::spawn_io(move || {
            let result = crate::backend::download::assets::scan_assets(
                &data_path,
                shared_path.as_deref(),
                instances_path.as_deref(),
            );
            let _ = sender_clone.send(AppMsg::AssetsReady(result));
        });
    }

    pub(crate) fn handle_assets_ready(&self, result: crate::backend::download::assets::AssetScanResult) {
        self.asset_view.emit(AssetInput::UpdateData(
            result,
            self.config.minecraft_data_path.clone(),
            self.config.shared_data_path.clone(),
            self.config.instances_path.clone(),
        ));
    }

    pub(crate) fn handle_asset_subpage_changed(&mut self, subtitle: Option<String>) {
        self.active_asset_subpage = subtitle;
    }

    pub(crate) fn handle_open_playtime(&mut self, sender: &ComponentSender<AppModel>) {
        self.active_sidebar_page = SidebarPage::Playtime;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Playtime));
        sender.input(AppMsg::RefreshPlaytime);
    }

    pub(crate) fn handle_refresh_playtime(&self, sender: &ComponentSender<AppModel>) {
        if self.config.is_demo {
            let manager = self.playtime_manager.clone();
            let mut instance_data = Vec::new();
            for inst in &self.instances {
                let playtime = manager.get_instance_playtime(&inst.id);
                instance_data.push((inst.id.clone(), inst.name.clone(), playtime, true));
            }
            sender.input(AppMsg::PlaytimeDataReady(manager, instance_data));
            return;
        }

        let sender_clone = sender.input_sender().clone();
        let instances = self.instances.clone();

        crate::backend::core::tasks::spawn_io(move || {
            let manager = crate::backend::playtime::PlaytimeManager::load();
            let mut instance_data = Vec::new();
            let mut seen_ids = std::collections::HashSet::new();

            for inst in &instances {
                seen_ids.insert(inst.id.clone());
                let playtime = manager.get_instance_playtime(&inst.id);
                instance_data.push((inst.id.clone(), inst.name.clone(), playtime, true));
            }

            // Include instances from history that are no longer on disk
            for (id, data) in &manager.instances {
                if !seen_ids.contains(id) {
                    instance_data.push((id.clone(), data.name.clone(), data.playtime, false));
                }
            }

            let _ = sender_clone.send(AppMsg::PlaytimeDataReady(manager, instance_data));
        });
    }

    pub(crate) fn handle_playtime_data_ready(
        &mut self,
        manager: crate::backend::playtime::PlaytimeManager,
        data: Vec<(String, String, u64, bool)>,
    ) {
        self.playtime_manager = manager.clone();
        self.playtime_view
            .emit(PlaytimeInput::UpdateData(manager, data));
    }

    pub(crate) fn handle_toggle_sidebar(&self) {
        let current = self.split_view.shows_sidebar();
        self.split_view.set_show_sidebar(!current);
    }

    pub(crate) fn handle_toggle_discover_search(&mut self) {
        self.discover_search_visible = !self.discover_search_visible;
    }

    pub(crate) fn handle_discover_search_changed(&mut self, query: String) {
        self.discover_search_query = query.clone();
        self.discover_view
            .emit(crate::frontend::views::discover::DiscoverInput::Search(query));
    }

    pub(crate) fn handle_refresh_discover(&self) {
        self.discover_view
            .emit(crate::frontend::views::discover::DiscoverInput::Refresh);
    }

    pub(crate) fn handle_sidebar_event(&mut self, sender: &ComponentSender<AppModel>, out: SidebarOutput) {
        match out {
            SidebarOutput::Navigate(page) => {
                self.active_sidebar_page = page;
                self.sidebar.emit(SidebarInput::SetSelected(page));
                self.discover_search_visible = false;
                if self.split_view.is_collapsed() {
                    self.split_view.set_show_sidebar(false);
                }

                match page {
                    SidebarPage::Library => self.selected_instance = None,
                    SidebarPage::Discover => self.selected_instance = None,
                    SidebarPage::Assets => sender.input(AppMsg::RefreshAssets),
                    SidebarPage::Playtime => sender.input(AppMsg::RefreshPlaytime),
                    SidebarPage::Accounts => {
                        self.selected_instance = None;
                        sender.input(AppMsg::RefreshAccountsRequest);
                    }
                    _ => {}
                }
            }
        }
    }

    pub(crate) fn handle_overview_event(&mut self, sender: &ComponentSender<AppModel>, out: OverviewOutput) {
        match out {
            OverviewOutput::SelectInstance(idx) => sender.input(AppMsg::SelectInstance(idx)),
            OverviewOutput::LaunchInstance(idx) => {
                sender.input(AppMsg::LaunchInstanceFromIndex(idx))
            }
            OverviewOutput::KillInstance(idx) => sender.input(AppMsg::KillInstanceFromIndex(idx)),
            OverviewOutput::RenameInstance(idx) => sender.input(AppMsg::RenameInstanceRequest(idx)),
            OverviewOutput::DeleteInstance(idx) => sender.input(AppMsg::DeleteInstanceRequest(idx)),
            OverviewOutput::MoveToGroupRequest(idx) => sender.input(AppMsg::MoveToGroupRequest(idx)),
            OverviewOutput::RemoveFromGroup(idx) => {
                sender.input(AppMsg::RemoveInstanceFromGroup(idx))
            }
            OverviewOutput::RenameGroup(name) => sender.input(AppMsg::RenameGroupRequest(name)),
            OverviewOutput::DeleteGroup(name) => sender.input(AppMsg::DeleteGroupRequest(name)),
            OverviewOutput::ChangeIconFromFile(idx) => {
                sender.input(AppMsg::ChangeInstanceIconFromFile(idx))
            }
            OverviewOutput::ApplyDefaultIcon(idx) => sender.input(AppMsg::ApplyDefaultIcon(idx)),
            OverviewOutput::ShareInstance(idx) => sender.input(AppMsg::ShareInstance(idx)),
            OverviewOutput::LayoutModeChanged(mode) => self.overview_layout = mode,
            OverviewOutput::AddInstance(tg) => sender.input(AppMsg::AddInstance(tg)),
            OverviewOutput::CreateGroup => sender.input(AppMsg::CreateGroupRequest),
            OverviewOutput::FolderChanged(folder_opt) => {
                self.current_folder = folder_opt.clone();
                let groups = self
                    .groups
                    .sorted_group_names()
                    .into_iter()
                    .map(String::from)
                    .collect();
                self.discover_view
                    .emit(crate::frontend::views::discover::DiscoverInput::UpdateGroups {
                        current_folder: folder_opt,
                        available_groups: groups,
                    });
            }
        }
    }

    pub(crate) fn handle_overview_back(&self) {
        self.overview_grid.emit(OverviewInput::GoBack);
    }

    pub(crate) fn handle_go_back(&self, sender: &ComponentSender<AppModel>) {
        if self.active_sidebar_page == SidebarPage::InstanceDetails {
            sender.input(AppMsg::ShowOverview);
        } else if self.active_sidebar_page == SidebarPage::Library {
            if self.current_folder.is_some() {
                sender.input(AppMsg::OverviewBack);
            }
        } else if self.active_sidebar_page == SidebarPage::Assets {
            if self.active_asset_subpage.is_some() {
                self.asset_view.emit(AssetInput::ShowCategoriesPage);
            }
        } else if self.active_sidebar_page == SidebarPage::Discover {
            if self.discover_details_open {
                self.discover_view
                    .emit(crate::frontend::views::discover::DiscoverInput::CloseDetails);
            }
        }
    }

    pub(crate) fn handle_show_overview(&mut self) {
        self.active_sidebar_page = SidebarPage::Library;
        self.selected_instance = None;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Library));
    }

    pub(crate) fn handle_set_narrow(&mut self, narrow: bool) {
        self.is_narrow = narrow;
        self.instance_summary.emit(SummaryInput::SetNarrow(narrow));
        self.overview_grid.emit(OverviewInput::SetNarrow(narrow));
    }

    pub(crate) fn handle_set_overview_layout(&mut self, mode: LayoutMode) {
        self.overview_layout = mode;
        self.overview_grid.emit(OverviewInput::SetLayoutMode(mode));
        self.config.preferred_view_type = match mode {
            LayoutMode::Grid => PreferredViewType::Grid,
            LayoutMode::List => PreferredViewType::List,
        };
        let _ = self.config.save();
    }

    pub(crate) fn handle_set_overview_sort_by(&mut self, sort_by: SortBy) {
        self.config.sort_by = sort_by;
        let _ = self.config.save();
        self.overview_grid.emit(OverviewInput::SetSortBy(sort_by));
    }

    pub(crate) fn handle_toggle_overview_layout(&self, sender: &ComponentSender<AppModel>) {
        let new_mode = if self.overview_layout == LayoutMode::Grid {
            LayoutMode::List
        } else {
            LayoutMode::Grid
        };
        sender.input(AppMsg::SetOverviewLayout(new_mode));
    }

    pub(crate) fn handle_switch_tab(&mut self, tab: String) {
        self.active_tab = tab;
    }
}
