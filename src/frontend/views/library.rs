use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::Instance;
use crate::config::SortBy;
use crate::frontend::views::instance::helpers::{self, ContextMenuOutput};
use adw::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn format_playtime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m", minutes)
    } else {
        "No playtime".to_string()
    }
}

fn loader_css_class(loader: &str) -> &'static str {
    match loader {
        "Fabric" => "overview-loader-fabric",
        "Forge" => "overview-loader-forge",
        "Quilt" => "overview-loader-quilt",
        "NeoForge" => "overview-loader-neoforge",
        _ => "overview-loader-generic",
    }
}

fn build_version_badge(version: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(version)
        .css_classes(vec![
            "overview-badge".to_string(),
            "overview-version-badge".to_string(),
        ])
        .build()
}

fn build_loader_badge(loader: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(loader)
        .css_classes(vec![
            "overview-badge".to_string(),
            loader_css_class(loader).to_string(),
        ])
        .build()
}

fn build_badges_box(inst: &Instance) -> gtk::Box {
    let badges = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(6)
        .build();

    if let Some(v) = &inst.minecraft_version {
        badges.append(&build_version_badge(v));
    }
    if let Some(l) = &inst.mod_loader {
        badges.append(&build_loader_badge(l));
    }
    badges
}

#[derive(Debug)]
pub enum OverviewInput {
    /// Full rebuild (called whenever instances or groups change).
    Rebuild(Vec<Instance>, InstanceGroups),
    /// Navigate into a group.
    SwitchToGroup(String),
    /// Navigate back to root.
    GoBack,
    /// A child in the flowbox was clicked.
    ChildActivated(gtk::FlowBoxChild),
    /// Responsive state change.
    SetNarrow(bool),
    /// Set layout mode.
    SetLayoutMode(LayoutMode),
    /// Set sort option.
    SetSortBy(SortBy),
    SetLoading(bool),
    ClearTextureCache(PathBuf),
}

#[derive(Debug)]
pub enum OverviewOutput {
    /// User activated an instance card.
    SelectInstance(usize),
    /// Request to rename an instance.
    RenameInstance(usize),
    /// Request to delete an instance.
    DeleteInstance(usize),
    /// Move instance into a group.
    MoveToGroupRequest(usize),
    /// Remove instance from group.
    RemoveFromGroup(usize),
    /// Create a new group.
    CreateGroup,
    /// Trigger "Add Instance" dialog.
    AddInstance,
    /// Rename a group.
    RenameGroup(String),
    /// Delete a group.
    DeleteGroup(String),
    /// Request to change an instance icon from file.
    ChangeIconFromFile(usize),
    /// Request to change an instance icon to default.
    ApplyDefaultIcon(usize),
    /// Request to share the instance.
    ShareInstance(usize),
    /// Notify that layout mode has changed.
    LayoutModeChanged(LayoutMode),
    /// Notify that the active group folder has changed.
    FolderChanged(Option<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Grid,
    List,
}

// ─── Internal state ───────────────────────────────────────────────────────────

/// What the grid is currently showing.
#[derive(Debug, Clone)]
enum GridView {
    /// Top-level: groups + ungrouped instances
    Root,
    /// Filtered to a single group
    Group(String),
}

pub struct OverviewGrid {
    instances: Vec<Instance>,
    groups: InstanceGroups,
    current_view: GridView,
    root_flow_box: gtk::FlowBox,
    group_flow_box: gtk::FlowBox,
    nav_stack: gtk::Stack,
    layout_mode: LayoutMode,
    sort_by: SortBy,
    loading: bool,
    popovers: std::cell::RefCell<Vec<gtk::Popover>>,
    texture_cache: std::cell::RefCell<HashMap<PathBuf, gtk::gdk::Texture>>,
}

// ─── Component ────────────────────────────────────────────────────────────────

#[relm4::component(pub)]
impl SimpleComponent for OverviewGrid {
    type Init = (LayoutMode, SortBy);
    type Input = OverviewInput;
    type Output = OverviewOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,
            set_css_classes: &["overview-root"],

            // Global separator to ensure main header always has a bottom border
            gtk::Separator {
                set_css_classes: &["header-separator"],
            },

            gtk::Stack {
                set_vexpand: true,
                set_hexpand: true,
                set_transition_type: gtk::StackTransitionType::Crossfade,

                add_named[Some("loading")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_spacing: 16,

                    adw::Spinner {
                        set_width_request: 64,
                        set_height_request: 64,
                    },

                    gtk::Label {
                        set_label: "Refreshing instances...",
                        set_css_classes: &["dim-label"],
                    }
                },

                add_named[Some("content")] = &gtk::Box {
                    set_vexpand: true,
                    set_hexpand: true,
                    #[local_ref]
                    nav_stack -> gtk::Stack {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                        set_transition_duration: 300,

                        add_named[Some("root")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,
                            set_vexpand: true,

                            gtk::ScrolledWindow {
                                set_vexpand: true,
                                set_hexpand: true,
                                set_hscrollbar_policy: gtk::PolicyType::Never,

                                adw::Clamp {
                                    set_maximum_size: 1100,
                                    set_tightening_threshold: 600,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_top: 32,
                                        set_margin_bottom: 32,
                                        set_spacing: 24,

                                        #[local_ref]
                                        root_flow_box -> gtk::FlowBox {
                                            #[watch]
                                            set_css_classes: if model.layout_mode == LayoutMode::List {
                                                &["overview-grid", "overview-list-mode"]
                                            } else {
                                                &["overview-grid", "overview-grid-mode"]
                                            },
                                            set_homogeneous: false,
                                            set_activate_on_single_click: true,
                                            #[watch]
                                            set_row_spacing: match model.layout_mode {
                                                LayoutMode::List => 8,
                                                LayoutMode::Grid => 24,
                                            },
                                            #[watch]
                                            set_column_spacing: 24,
                                            set_margin_start: 32,
                                            set_margin_end: 32,
                                            set_selection_mode: gtk::SelectionMode::Single,
                                            set_can_focus: true,
                                            set_focusable: true,
                                            #[watch]
                                            set_max_children_per_line: match model.layout_mode {
                                                LayoutMode::List => 1,
                                                LayoutMode::Grid => 8,
                                            },
                                            set_min_children_per_line: 1,
                                            set_valign: gtk::Align::Start,
                                        }
                                    }
                                }
                            }
                        },

                        add_named[Some("group")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,
                            set_vexpand: true,

                            gtk::ScrolledWindow {
                                set_vexpand: true,
                                set_hexpand: true,
                                set_hscrollbar_policy: gtk::PolicyType::Never,

                                adw::Clamp {
                                    set_maximum_size: 1100,
                                    set_tightening_threshold: 600,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_top: 32,
                                        set_margin_bottom: 32,
                                        set_spacing: 24,

                                        #[local_ref]
                                        group_flow_box -> gtk::FlowBox {
                                            #[watch]
                                            set_css_classes: if model.layout_mode == LayoutMode::List {
                                                &["overview-grid", "overview-list-mode"]
                                            } else {
                                                &["overview-grid", "overview-grid-mode"]
                                            },
                                            set_homogeneous: false,
                                            set_activate_on_single_click: true,
                                            #[watch]
                                            set_row_spacing: match model.layout_mode {
                                                LayoutMode::List => 8,
                                                LayoutMode::Grid => 24,
                                            },
                                            #[watch]
                                            set_column_spacing: 24,
                                            set_margin_start: 32,
                                            set_margin_end: 32,
                                            set_selection_mode: gtk::SelectionMode::Single,
                                            set_can_focus: true,
                                            set_focusable: true,
                                            #[watch]
                                            set_max_children_per_line: match model.layout_mode {
                                                LayoutMode::List => 1,
                                                LayoutMode::Grid => 8,
                                            },
                                            set_min_children_per_line: 1,
                                            set_valign: gtk::Align::Start,
                                        }
                                    }
                                }
                            },
                        },
                    }
                },

                add_named[Some("empty")] = &adw::StatusPage {
                    set_title: "No Instances Found",
                    set_description: Some("Create a new instance to get started!"),
                    set_icon_name: Some("application-x-executable-symbolic"),
                    set_vexpand: true,

                    #[wrap(Some)]
                    set_child = &gtk::Button {
                        set_label: "Create Instance",
                        set_halign: gtk::Align::Center,
                        set_css_classes: &["suggested-action", "pill"],
                        connect_clicked[sender] => move |_| {
                            sender.output(OverviewOutput::AddInstance).unwrap();
                        }
                    }
                },

                // Must come after add_named so children exist on first render
                #[watch]
                set_visible_child_name: if model.loading {
                    "loading"
                } else if model.instances.is_empty() {
                    "empty"
                } else {
                    "content"
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let (layout_mode, sort_by) = init;
        let root_flow_box = gtk::FlowBox::new();
        let group_flow_box = gtk::FlowBox::new();
        let nav_stack = gtk::Stack::new();

        // Connect signal handlers for BOTH flowboxes
        {
            let s = sender.clone();
            root_flow_box.connect_child_activated(move |_, child| {
                s.input(OverviewInput::ChildActivated(child.clone()));
            });
            let s2 = sender.clone();
            group_flow_box.connect_child_activated(move |_, child| {
                s2.input(OverviewInput::ChildActivated(child.clone()));
            });
        }

        let model = OverviewGrid {
            instances: Vec::new(),
            groups: InstanceGroups::default(),
            current_view: GridView::Root,
            root_flow_box: root_flow_box.clone(),
            group_flow_box: group_flow_box.clone(),
            nav_stack: nav_stack.clone(),
            layout_mode,
            sort_by,
            loading: true,
            popovers: std::cell::RefCell::new(Vec::new()),
            texture_cache: std::cell::RefCell::new(HashMap::new()),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OverviewInput, sender: ComponentSender<Self>) {
        match msg {
            OverviewInput::Rebuild(instances, groups) => {
                self.instances = instances;
                self.groups = groups;
                self.loading = false;
                if let GridView::Group(ref gname) = self.current_view {
                    if !self.groups.groups.contains_key(gname.as_str()) {
                        self.current_view = GridView::Root;
                    }
                }
                self.rebuild_grid(&sender);
            }
            OverviewInput::SwitchToGroup(name) => {
                self.current_view = GridView::Group(name);
                self.rebuild_grid(&sender);
            }
            OverviewInput::GoBack => {
                self.current_view = GridView::Root;
                self.rebuild_grid(&sender);
            }
            OverviewInput::ChildActivated(child) => {
                let idx = child.index();
                if let GridView::Root = self.current_view {
                    let group_count = self.groups.groups.len();
                    if (idx as usize) < group_count {
                        let gname = self.groups.sorted_group_names()[idx as usize].to_string();
                        sender.input(OverviewInput::SwitchToGroup(gname));
                    } else {
                        let mut ungrouped: Vec<(usize, &Instance)> = self
                            .instances
                            .iter()
                            .enumerate()
                            .filter(|(_, inst)| {
                                let folder =
                                    inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                self.groups.get_instance_group(folder).is_none()
                            })
                            .collect();
                        self.sort_instances(&mut ungrouped);

                        let ungrouped_idx = idx as usize - group_count;
                        if let Some(&(flat_idx, _)) = ungrouped.get(ungrouped_idx) {
                            sender.output(OverviewOutput::SelectInstance(flat_idx)).ok();
                        }
                    }
                } else if let GridView::Group(ref gname) = self.current_view {
                    let info = &self.groups.groups[gname];
                    let mut members: Vec<(usize, &Instance)> = self
                        .instances
                        .iter()
                        .enumerate()
                        .filter(|(_, inst)| {
                            let folder =
                                inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            info.instances.contains(folder)
                        })
                        .collect();
                    self.sort_instances(&mut members);

                    if let Some(&(flat_idx, _)) = members.get(idx as usize) {
                        sender.output(OverviewOutput::SelectInstance(flat_idx)).ok();
                    }
                }
            }
            OverviewInput::SetNarrow(narrow) => {
                if self.layout_mode == LayoutMode::Grid && narrow {
                    self.layout_mode = LayoutMode::List;
                    self.rebuild_grid(&sender);
                    sender
                        .output(OverviewOutput::LayoutModeChanged(LayoutMode::List))
                        .ok();
                }
            }
            OverviewInput::SetLayoutMode(mode) => {
                self.layout_mode = mode;
                self.rebuild_grid(&sender);
            }
            OverviewInput::SetSortBy(sort_by) => {
                self.sort_by = sort_by;
                self.rebuild_grid(&sender);
            }
            OverviewInput::SetLoading(loading) => {
                self.loading = loading;
            }
            OverviewInput::ClearTextureCache(path) => {
                self.texture_cache.borrow_mut().remove(&path);
            }
        }
    }
}

// ─── Grid builder ─────────────────────────────────────────────────────────────

impl OverviewGrid {
    fn sort_instances(&self, list: &mut Vec<(usize, &Instance)>) {
        match self.sort_by {
            SortBy::Alphabetical => {
                list.sort_by(|a, b| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()));
            }
            SortBy::LastPlayed => {
                list.sort_by(|a, b| {
                    let a_time = a.1.last_launched.unwrap_or(0);
                    let b_time = b.1.last_launched.unwrap_or(0);
                    b_time
                        .cmp(&a_time)
                        .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
                });
            }
            SortBy::Playtime => {
                list.sort_by(|a, b| {
                    let a_play = a.1.total_time_played;
                    let b_play = b.1.total_time_played;
                    b_play
                        .cmp(&a_play)
                        .then_with(|| a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase()))
                });
            }
        }
    }

    fn rebuild_grid(&self, _sender: &ComponentSender<Self>) {
        for pop in self.popovers.borrow_mut().drain(..) {
            pop.unparent();
        }

        match &self.current_view.clone() {
            GridView::Root => {
                self.nav_stack.set_visible_child_name("root");

                while let Some(child) = self.root_flow_box.first_child() {
                    self.root_flow_box.remove(&child);
                }
                self.build_root_view(&self.root_flow_box, _sender);
                _sender.output(OverviewOutput::FolderChanged(None)).ok();
            }
            GridView::Group(gname) => {
                self.nav_stack.set_visible_child_name("group");

                while let Some(child) = self.group_flow_box.first_child() {
                    self.group_flow_box.remove(&child);
                }
                self.build_group_view(gname, &self.group_flow_box, _sender);
                _sender
                    .output(OverviewOutput::FolderChanged(Some(gname.clone())))
                    .ok();
            }
        }
    }

    fn build_root_view(&self, flow_box: &gtk::FlowBox, _sender: &ComponentSender<Self>) {
        for gname in self.groups.sorted_group_names() {
            let info = &self.groups.groups[gname];
            let count = info.instances.len();
            let card = self.build_folder_card(gname, count, _sender);
            flow_box.append(&card);
        }

        let mut ungrouped_instances: Vec<(usize, &Instance)> = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, inst)| {
                let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                self.groups.get_instance_group(folder).is_none()
            })
            .collect();

        self.sort_instances(&mut ungrouped_instances);

        for (flat_idx, inst) in ungrouped_instances {
            let card = self.build_instance_card(flat_idx, inst, _sender);
            flow_box.append(&card);
        }
    }

    fn build_group_view(
        &self,
        gname: &str,
        flow_box: &gtk::FlowBox,
        _sender: &ComponentSender<Self>,
    ) {
        let info = match self.groups.groups.get(gname) {
            Some(i) => i,
            None => return,
        };

        let mut group_instances: Vec<(usize, &Instance)> = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, inst)| {
                let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                info.instances.contains(folder)
            })
            .collect();

        self.sort_instances(&mut group_instances);

        for (flat_idx, inst) in group_instances {
            let card = self.build_instance_card(flat_idx, inst, _sender);
            flow_box.append(&card);
        }
    }

    fn build_folder_card(
        &self,
        name: &str,
        count: usize,
        sender: &ComponentSender<Self>,
    ) -> gtk::FlowBoxChild {
        let child = gtk::FlowBoxChild::builder()
            .css_classes(vec!["overview-card-child"])
            .focusable(true)
            .build();

        if self.layout_mode == LayoutMode::List {
            let card = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(12)
                .css_classes(vec!["overview-list-card"])
                .build();

            let icon = gtk::Image::builder()
                .icon_name("folder-symbolic")
                .pixel_size(32)
                .margin_start(4)
                .build();
            card.append(&icon);

            let txt_box = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(2)
                .hexpand(true)
                .valign(gtk::Align::Center)
                .build();
            let lbl = gtk::Label::builder()
                .label(name)
                .halign(gtk::Align::Start)
                .css_classes(vec!["overview-card-title"])
                .build();
            let sub = gtk::Label::builder()
                .label(&format!("{} instances", count))
                .halign(gtk::Align::Start)
                .css_classes(vec!["dim-label", "caption"])
                .build();
            txt_box.append(&lbl);
            txt_box.append(&sub);
            card.append(&txt_box);

            let arrow = gtk::Image::builder()
                .icon_name("go-next-symbolic")
                .css_classes(vec!["dim-label"])
                .margin_end(4)
                .build();
            card.append(&arrow);

            let menu_btn = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .css_classes(vec!["flat"])
                .valign(gtk::Align::Center)
                .margin_end(8)
                .build();
            card.append(&menu_btn);

            self.attach_group_context_menu(&child, Some(&menu_btn), name.to_string(), sender);

            child.set_child(Some(&card));
            return child;
        }

        let card_width = 180;
        let card_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .build();

        let icon_box = gtk::Box::builder()
            .halign(gtk::Align::Center)
            .margin_top(20)
            .build();
        let icon = gtk::Image::builder()
            .icon_name("folder-symbolic")
            .pixel_size(64)
            .build();
        icon_box.append(&icon);
        card_content.append(&icon_box);

        let info_area = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_bottom(20)
            .build();

        let title = gtk::Label::builder()
            .label(name)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(vec!["overview-card-title"])
            .halign(gtk::Align::Center)
            .build();
        info_area.append(&title);

        let sub = gtk::Label::builder()
            .label(&format!("{} instances", count))
            .css_classes(vec!["overview-card-subtitle"])
            .halign(gtk::Align::Center)
            .build();
        info_area.append(&sub);

        card_content.append(&info_area);

        let card = gtk::Overlay::builder()
            .child(&card_content)
            .css_classes(vec!["overview-card", "overview-folder-card"])
            .width_request(card_width)
            .build();

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .css_classes(vec!["flat", "circular"])
            .valign(gtk::Align::Start)
            .halign(gtk::Align::End)
            .margin_top(4)
            .margin_end(4)
            .build();
        card.add_overlay(&menu_btn);

        self.attach_group_context_menu(&child, Some(&menu_btn), name.to_string(), sender);

        child.set_child(Some(&card));
        child
    }

    fn build_instance_card(
        &self,
        flat_idx: usize,
        inst: &Instance,
        sender: &ComponentSender<Self>,
    ) -> gtk::FlowBoxChild {
        let child = gtk::FlowBoxChild::builder()
            .css_classes(vec!["overview-card-child"])
            .focusable(true)
            .build();

        if self.layout_mode == LayoutMode::List {
            // ── Card-backed list row ─────────────────────────────────────
            let card = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(12)
                .css_classes(vec!["overview-list-card"])
                .build();

            let icon = self.get_instance_icon(inst, 40);
            icon.set_margin_start(4);
            card.append(&icon);

            // Text + badges column
            let text_col = gtk::Box::builder()
                .orientation(gtk::Orientation::Vertical)
                .spacing(4)
                .hexpand(true)
                .valign(gtk::Align::Center)
                .build();

            let title = gtk::Label::builder()
                .label(&inst.name)
                .halign(gtk::Align::Start)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .css_classes(vec!["overview-card-title"])
                .build();
            text_col.append(&title);

            // Badges row
            let badges = build_badges_box(inst);
            badges.set_halign(gtk::Align::Start);
            text_col.append(&badges);

            card.append(&text_col);

            // Right-side playtime highlight
            if inst.total_time_played > 0 {
                let pt_label = gtk::Label::builder()
                    .label(&format_playtime(inst.total_time_played))
                    .css_classes(vec!["dim-label", "caption"])
                    .valign(gtk::Align::Center)
                    .margin_end(4)
                    .build();
                card.append(&pt_label);
            }

            let menu_btn = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .css_classes(vec!["flat"])
                .valign(gtk::Align::Center)
                .margin_end(8)
                .build();
            card.append(&menu_btn);

            self.attach_instance_context_menu(&child, Some(&menu_btn), flat_idx, inst, sender);
            child.set_child(Some(&card));
            return child;
        }

        // ── Grid Card ────────────────────────────────────────────────────────
        let card_width = 180;
        let card_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        let icon_area = gtk::Box::builder()
            .margin_top(24)
            .margin_bottom(12)
            .halign(gtk::Align::Center)
            .build();
        let icon = self.get_instance_icon(inst, 64);
        icon_area.append(&icon);
        card_content.append(&icon_area);

        let info_area = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(16)
            .build();

        let title = gtk::Label::builder()
            .label(&inst.name)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .css_classes(vec!["overview-card-title"])
            .halign(gtk::Align::Center)
            .build();
        info_area.append(&title);

        // Badges row (centered)
        let badges = build_badges_box(inst);
        badges.set_halign(gtk::Align::Center);
        info_area.append(&badges);

        // Stats line
        let mut stat_parts: Vec<String> = Vec::new();
        if inst.total_time_played > 0 {
            stat_parts.push(format_playtime(inst.total_time_played));
        }
        if !inst.mods.is_empty() {
            stat_parts.push(format!("{} mods", inst.mods.len()));
        }
        if !stat_parts.is_empty() {
            let stats_label = gtk::Label::builder()
                .label(&stat_parts.join(" · "))
                .css_classes(vec!["overview-card-stats"])
                .halign(gtk::Align::Center)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            info_area.append(&stats_label);
        }

        card_content.append(&info_area);

        let card = gtk::Overlay::builder()
            .child(&card_content)
            .css_classes(vec!["overview-card"])
            .width_request(card_width)
            .build();

        let menu_btn = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .css_classes(vec!["flat", "circular"])
            .valign(gtk::Align::Start)
            .halign(gtk::Align::End)
            .margin_top(4)
            .margin_end(4)
            .build();
        card.add_overlay(&menu_btn);

        self.attach_instance_context_menu(&child, Some(&menu_btn), flat_idx, inst, sender);

        child.set_child(Some(&card));
        child
    }

    fn attach_instance_context_menu(
        &self,
        widget: &impl IsA<gtk::Widget>,
        menu_btn: Option<&gtk::MenuButton>,
        flat_idx: usize,
        inst: &Instance,
        sender: &ComponentSender<Self>,
    ) {
        let folder_name = inst
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let s_clone = sender.clone();

        let parent_widget = if menu_btn.is_some() {
            None
        } else {
            Some(widget.upcast_ref::<gtk::Widget>())
        };
        let popover = helpers::create_instance_popover(
            flat_idx,
            inst,
            self.groups.get_instance_group(&folder_name).is_some(),
            move |out| {
                match out {
                    ContextMenuOutput::SelectInstance(idx) => {
                        s_clone.output(OverviewOutput::SelectInstance(idx)).ok()
                    }
                    ContextMenuOutput::RenameInstance(idx) => {
                        s_clone.output(OverviewOutput::RenameInstance(idx)).ok()
                    }
                    ContextMenuOutput::DeleteInstance(idx) => {
                        s_clone.output(OverviewOutput::DeleteInstance(idx)).ok()
                    }
                    ContextMenuOutput::MoveToGroupRequest(idx) => {
                        s_clone.output(OverviewOutput::MoveToGroupRequest(idx)).ok()
                    }
                    ContextMenuOutput::RemoveFromGroup(idx) => {
                        s_clone.output(OverviewOutput::RemoveFromGroup(idx)).ok()
                    }
                    ContextMenuOutput::ChangeIconFromFile(idx) => {
                        s_clone.output(OverviewOutput::ChangeIconFromFile(idx)).ok()
                    }
                    ContextMenuOutput::ApplyDefaultIcon(idx) => {
                        s_clone.output(OverviewOutput::ApplyDefaultIcon(idx)).ok()
                    }
                    ContextMenuOutput::ShareInstance(idx) => {
                        s_clone.output(OverviewOutput::ShareInstance(idx)).ok()
                    }
                    _ => None,
                };
            },
            parent_widget,
        );

        if let Some(mb) = menu_btn {
            mb.set_popover(Some(&popover));
        }

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let pop = popover.clone();
            let w_ref = widget.clone();
            pop.connect_map(move |_| {
                w_ref.add_css_class("menu-open");
            });
            let w_ref2 = widget.clone();
            pop.connect_closed(move |p| {
                w_ref2.remove_css_class("menu-open");
                p.set_pointing_to(None);
            });

            let w_ref3 = widget.clone();
            let mb_clone = menu_btn.cloned();
            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);

                if let Some(mb) = &mb_clone {
                    if let Some(pt) =
                        w_ref3.compute_point(mb, &gtk::graphene::Point::new(x as f32, y as f32))
                    {
                        pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            pt.x() as i32,
                            pt.y() as i32,
                            1,
                            1,
                        )));
                    } else {
                        pop.set_pointing_to(None);
                    }
                } else {
                    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                }

                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
        self.popovers.borrow_mut().push(popover);
    }

    fn attach_group_context_menu(
        &self,
        widget: &impl IsA<gtk::Widget>,
        menu_btn: Option<&gtk::MenuButton>,
        group_name: String,
        sender: &ComponentSender<Self>,
    ) {
        let s_clone = sender.clone();

        let parent_widget = if menu_btn.is_some() {
            None
        } else {
            Some(widget.upcast_ref::<gtk::Widget>())
        };
        let popover = helpers::create_group_popover(
            group_name,
            move |out| {
                match out {
                    ContextMenuOutput::RenameGroup(name) => {
                        s_clone.output(OverviewOutput::RenameGroup(name)).ok()
                    }
                    ContextMenuOutput::DeleteGroup(name) => {
                        s_clone.output(OverviewOutput::DeleteGroup(name)).ok()
                    }
                    _ => None,
                };
            },
            parent_widget,
        );

        if let Some(mb) = menu_btn {
            mb.set_popover(Some(&popover));
        }

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let pop = popover.clone();
            let w_ref = widget.clone();
            pop.connect_map(move |_| {
                w_ref.add_css_class("menu-open");
            });
            let w_ref2 = widget.clone();
            pop.connect_closed(move |p| {
                w_ref2.remove_css_class("menu-open");
                p.set_pointing_to(None);
            });

            let w_ref3 = widget.clone();
            let mb_clone = menu_btn.cloned();
            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);

                if let Some(mb) = &mb_clone {
                    if let Some(pt) =
                        w_ref3.compute_point(mb, &gtk::graphene::Point::new(x as f32, y as f32))
                    {
                        pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                            pt.x() as i32,
                            pt.y() as i32,
                            1,
                            1,
                        )));
                    } else {
                        pop.set_pointing_to(None);
                    }
                } else {
                    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                }

                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
        self.popovers.borrow_mut().push(popover);
    }
    fn get_instance_icon(&self, inst: &Instance, size: i32) -> gtk::Image {
        let mut icon_path = inst.path.join("icon.png");
        if !icon_path.exists() {
            icon_path = inst.minecraft_dir.join("icon.png");
        }

        if icon_path.exists() {
            let mut cache = self.texture_cache.borrow_mut();
            if let Some(texture) = cache.get(&icon_path) {
                return gtk::Image::builder()
                    .paintable(texture)
                    .pixel_size(size)
                    .build();
            }

            if let Ok(texture) = gtk::gdk::Texture::from_filename(&icon_path) {
                cache.insert(icon_path, texture.clone());
                return gtk::Image::builder()
                    .paintable(&texture)
                    .pixel_size(size)
                    .build();
            }
        }

        gtk::Image::builder()
            .icon_name("application-x-executable-symbolic")
            .pixel_size(size)
            .build()
    }
}
