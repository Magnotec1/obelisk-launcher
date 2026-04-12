use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::{Instance, ModLoader};
use crate::frontend::views::instance::helpers::{self, ContextMenuOutput};
use adw::prelude::*;
use relm4::prelude::*;

// ─── Public types ─────────────────────────────────────────────────────────────

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
    SetLoading(bool),
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
    /// Request to change an instance icon.
    ChangeIcon(usize),
    /// Request to share the instance.
    ShareInstance(usize),
    /// Notify that layout mode has changed.
    LayoutModeChanged(LayoutMode),
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
    flow_box: gtk::FlowBox,
    header_bar: gtk::Box, // Back button area (hidden at root)
    group_label: gtk::Label,
    layout_mode: LayoutMode,
    loading: bool,
    popovers: std::cell::RefCell<Vec<gtk::Popover>>,
}

// ─── Component ────────────────────────────────────────────────────────────────

#[relm4::component(pub)]
impl SimpleComponent for OverviewGrid {
    type Init = ();
    type Input = OverviewInput;
    type Output = OverviewOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,
            set_css_classes: &["overview-root"],

            // Back bar (only visible when inside a group)
            #[local_ref]
            header_bar -> gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_start: 24,
                set_margin_end: 24,
                set_margin_top: 16,
                set_margin_bottom: 0,
                set_visible: false,
                set_css_classes: &["overview-back-bar"],

                #[local_ref]
                back_btn -> gtk::Button {
                    set_icon_name: "go-previous-symbolic",
                    set_has_frame: false,
                    set_has_tooltip: true,
                    set_tooltip_text: Some("Back to all instances"),
                    set_css_classes: &["overview-back-btn", "circular"],
                },

                #[local_ref]
                group_label -> gtk::Label {
                    set_css_classes: &["title-2"],
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                },
            },

            gtk::Stack {
                set_vexpand: true,
                set_transition_type: gtk::StackTransitionType::Crossfade,
                #[watch]
                set_visible_child_name: if model.loading { 
                    "loading" 
                } else if model.instances.is_empty() { 
                    "empty" 
                } else { 
                    "content" 
                },

                add_named[Some("loading")] = &adw::Spinner {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_width_request: 32,
                    set_height_request: 32,
                },

                add_named[Some("content")] = &gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    #[local_ref]
                    flow_box -> gtk::FlowBox {
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
                            LayoutMode::List => 4,
                            LayoutMode::Grid => 24,
                        },
                        #[watch]
                        set_column_spacing: 24,
                        set_margin_start: 24,
                        set_margin_end: 24,
                        set_margin_top: 24,
                        set_margin_bottom: 24,
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
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let flow_box = gtk::FlowBox::new();
        let header_bar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let back_btn = gtk::Button::new();
        let group_label = gtk::Label::new(None);

        // Connect signal handlers ONCE in init
        {
            let s = sender.clone();
            flow_box.connect_child_activated(move |_, child| {
                s.input(OverviewInput::ChildActivated(child.clone()));
            });
        }
        {
            let s = sender.clone();
            back_btn.connect_clicked(move |_| {
                s.input(OverviewInput::GoBack);
            });
        }

        let model = OverviewGrid {
            instances: Vec::new(),
            groups: InstanceGroups::default(),
            current_view: GridView::Root,
            flow_box: flow_box.clone(),
            header_bar: header_bar.clone(),
            group_label: group_label.clone(),
            layout_mode: LayoutMode::Grid,
            loading: false,
            popovers: std::cell::RefCell::new(Vec::new()),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OverviewInput, sender: ComponentSender<Self>) {
        match msg {
            OverviewInput::Rebuild(instances, groups) => {
                self.instances = instances;
                self.groups = groups;
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
                        let ungrouped_idx = idx as usize - group_count;
                        let mut ungrouped_found = 0;
                        for (flat_idx, inst) in self.instances.iter().enumerate() {
                            let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            if self.groups.get_instance_group(folder).is_none() {
                                if ungrouped_found == ungrouped_idx {
                                    sender.output(OverviewOutput::SelectInstance(flat_idx)).ok();
                                    break;
                                }
                                ungrouped_found += 1;
                            }
                        }
                    }
                } else if let GridView::Group(ref gname) = self.current_view {
                    let info = &self.groups.groups[gname];
                    let mut members: Vec<usize> = self.instances.iter().enumerate()
                        .filter(|(_, inst)| {
                            let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                            info.instances.contains(folder)
                        })
                        .map(|(i, _)| i)
                        .collect();
                    let insts = &self.instances;
                    members.sort_by(|&a, &b| insts[a].name.cmp(&insts[b].name));

                    if let Some(&flat_idx) = members.get(idx as usize) {
                        sender.output(OverviewOutput::SelectInstance(flat_idx)).ok();
                    }
                }
            }
            OverviewInput::SetNarrow(narrow) => {
                if self.layout_mode == LayoutMode::Grid && narrow {
                    self.layout_mode = LayoutMode::List;
                    self.rebuild_grid(&sender);
                    sender.output(OverviewOutput::LayoutModeChanged(LayoutMode::List)).ok();
                }
            }
            OverviewInput::SetLayoutMode(mode) => {
                self.layout_mode = mode;
                self.rebuild_grid(&sender);
            }
            OverviewInput::SetLoading(loading) => {
                self.loading = loading;
            }
        }
    }
}

// ─── Grid builder ─────────────────────────────────────────────────────────────

impl OverviewGrid {
    fn rebuild_grid(&self, _sender: &ComponentSender<Self>) {
        while let Some(child) = self.flow_box.first_child() {
            self.flow_box.remove(&child);
        }
        for pop in self.popovers.borrow_mut().drain(..) {
            pop.unparent();
        }

        match &self.current_view.clone() {
            GridView::Root => {
                self.header_bar.set_visible(false);
                self.build_root_view(_sender);
            }
            GridView::Group(gname) => {
                self.header_bar.set_visible(true);
                self.group_label.set_label(gname);
                self.build_group_view(gname, _sender);
            }
        }
    }

    fn build_root_view(&self, _sender: &ComponentSender<Self>) {
        for gname in self.groups.sorted_group_names() {
            let info = &self.groups.groups[gname];
            let count = info.instances.len();
            let card = self.build_folder_card(gname, count, _sender);
            self.flow_box.append(&card);
        }

        for (flat_idx, inst) in self.instances.iter().enumerate() {
            let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if self.groups.get_instance_group(folder).is_none() {
                let card = self.build_instance_card(flat_idx, inst, _sender);
                self.flow_box.append(&card);
            }
        }
    }

    fn build_group_view(&self, gname: &str, _sender: &ComponentSender<Self>) {
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
        group_instances.sort_by(|a, b| a.1.name.cmp(&b.1.name));

        for (flat_idx, inst) in group_instances {
            let card = self.build_instance_card(flat_idx, inst, _sender);
            self.flow_box.append(&card);
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
                .css_classes(vec!["overview-list-row"])
                .build();
            card.set_margin_all(8);
            
            let icon = gtk::Image::builder()
                .icon_name("folder-symbolic")
                .pixel_size(24)
                .build();
            card.append(&icon);

            let txt_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let lbl = gtk::Label::builder()
                .label(name)
                .halign(gtk::Align::Start)
                .build();
            let sub = gtk::Label::builder()
                .label(&format!("{} instances", count))
                .halign(gtk::Align::Start)
                .css_classes(vec!["dim-label", "caption"])
                .build();
            txt_box.append(&lbl);
            txt_box.append(&sub);
            txt_box.set_hexpand(true);
            card.append(&txt_box);

            self.attach_group_context_menu(&child, name.to_string(), sender);

            child.set_child(Some(&card));
            return child;
        }

        let card_width = 180;
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .css_classes(vec!["overview-card", "overview-folder-card"])
            .width_request(card_width)
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
        card.append(&icon_box);

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

        card.append(&info_area);

        self.attach_group_context_menu(&child, name.to_string(), sender);

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
             let card = gtk::Box::builder()
                .orientation(gtk::Orientation::Horizontal)
                .spacing(12)
                .css_classes(vec!["overview-list-row"])
                .build();
            card.set_margin_all(8);
            
            let icon = get_instance_icon(inst, 32);
            card.append(&icon);

            let txt_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let lbl = gtk::Label::builder()
                .label(&inst.name)
                .halign(gtk::Align::Start)
                .build();
            
            let mut sub_parts = Vec::new();
            if let Some(v) = &inst.minecraft_version { sub_parts.push(v.clone()); }
            if let Some(l) = &inst.mod_loader { sub_parts.push(format!("{:?}", l)); }
            let sub_text = sub_parts.join(" · ");

            let sub = gtk::Label::builder()
                .label(&sub_text)
                .halign(gtk::Align::Start)
                .css_classes(vec!["dim-label", "caption"])
                .build();
            txt_box.append(&lbl);
            txt_box.append(&sub);
            txt_box.set_hexpand(true);
            card.append(&txt_box);

            self.attach_instance_context_menu(&child, flat_idx, inst, sender);

            child.set_child(Some(&card));
            return child;
        }

        // ── Grid Card Rework ─────────────────────────────────────────────────
        let card_width = 180;
        let card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .css_classes(vec!["overview-card", "overview-instance-card"])
            .width_request(card_width)
            .build();
        
        let icon_area = gtk::Box::builder()
            .margin_top(24)
            .margin_bottom(12)
            .halign(gtk::Align::Center)
            .build();
        let icon = get_instance_icon(inst, 64);
        icon_area.append(&icon);
        card.append(&icon_area);

        let info_area = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(24)
            .build();

        let title = gtk::Label::builder()
            .label(&inst.name)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(15)
            .css_classes(vec!["overview-card-title"])
            .halign(gtk::Align::Center)
            .build();
        info_area.append(&title);

        // Badges row
        let badges_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk::Align::Center)
            .build();

        if let Some(v) = &inst.minecraft_version {
            let v_badge = gtk::Label::builder()
                .label(v)
                .css_classes(vec!["overview-badge", "overview-version-badge"])
                .build();
            badges_box.append(&v_badge);
        }

        let (loader, _) = inst.get_loader_info();
        if loader != ModLoader::None {
            let loader_class = match loader {
                ModLoader::Fabric => "overview-loader-fabric",
                ModLoader::Forge => "overview-loader-forge",
                ModLoader::Quilt => "overview-loader-quilt",
                ModLoader::NeoForge => "overview-loader-neoforge",
                _ => "overview-loader-generic",
            };
            let l_badge = gtk::Label::builder()
                .label(&format!("{:?}", loader))
                .css_classes(vec!["overview-badge", loader_class])
                .build();
            badges_box.append(&l_badge);
        }
        info_area.append(&badges_box);

        let time_str = format_time_played(inst.total_time_played);
        if !time_str.is_empty() {
            let stats_label = gtk::Label::builder()
                .label(&time_str)
                .css_classes(vec!["overview-card-stats"])
                .halign(gtk::Align::Center)
                .build();
            info_area.append(&stats_label);
        }

        card.append(&info_area);
        self.attach_instance_context_menu(&child, flat_idx, inst, sender);

        child.set_child(Some(&card));
        child
    }

    fn attach_instance_context_menu(&self, widget: &impl IsA<gtk::Widget>, flat_idx: usize, inst: &Instance, sender: &ComponentSender<Self>) {
        let folder_name = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let s_clone = sender.clone();
        
        let w_link = widget.clone();
        let popover = helpers::create_instance_popover(flat_idx, inst, self.groups.get_instance_group(&folder_name).is_some(), move |out| {
            match out {
                ContextMenuOutput::SelectInstance(idx) => s_clone.output(OverviewOutput::SelectInstance(idx)).ok(),
                ContextMenuOutput::RenameInstance(idx) => s_clone.output(OverviewOutput::RenameInstance(idx)).ok(),
                ContextMenuOutput::DeleteInstance(idx) => s_clone.output(OverviewOutput::DeleteInstance(idx)).ok(),
                ContextMenuOutput::MoveToGroupRequest(idx) => s_clone.output(OverviewOutput::MoveToGroupRequest(idx)).ok(),
                ContextMenuOutput::RemoveFromGroup(idx) => s_clone.output(OverviewOutput::RemoveFromGroup(idx)).ok(),
                ContextMenuOutput::ChangeIcon(idx) => s_clone.output(OverviewOutput::ChangeIcon(idx)).ok(),
                ContextMenuOutput::ShareInstance(idx) => s_clone.output(OverviewOutput::ShareInstance(idx)).ok(),
                _ => None,
            };
        }, Some(&w_link));

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let pop = popover.clone();
            let w_ref = widget.clone();
            pop.connect_map(move |_| {
                w_ref.add_css_class("menu-open");
            });
            let w_ref2 = widget.clone();
            pop.connect_closed(move |_| {
                w_ref2.remove_css_class("menu-open");
            });

            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
        self.popovers.borrow_mut().push(popover);
    }

    fn attach_group_context_menu(&self, widget: &impl IsA<gtk::Widget>, group_name: String, sender: &ComponentSender<Self>) {
        let s_clone = sender.clone();
        let w_link = widget.clone();
        let popover = helpers::create_group_popover(group_name, move |out| {
            match out {
                ContextMenuOutput::RenameGroup(name) => s_clone.output(OverviewOutput::RenameGroup(name)).ok(),
                ContextMenuOutput::DeleteGroup(name) => s_clone.output(OverviewOutput::DeleteGroup(name)).ok(),
                _ => None,
            };
        }, Some(&w_link));

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let pop = popover.clone();
            let w_ref = widget.clone();
            pop.connect_map(move |_| {
                w_ref.add_css_class("menu-open");
            });
            let w_ref2 = widget.clone();
            pop.connect_closed(move |_| {
                w_ref2.remove_css_class("menu-open");
            });

            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
        self.popovers.borrow_mut().push(popover);
    }
}

fn get_instance_icon(inst: &Instance, size: i32) -> gtk::Image {
    let icon_path = inst.minecraft_dir.join("icon.png");

    if icon_path.exists() {
        if let Ok(texture) = gtk::gdk::Texture::from_filename(&icon_path) {
            return gtk::Image::builder().paintable(&texture).pixel_size(size).build();
        }
    }

    gtk::Image::builder().icon_name("application-x-executable-symbolic").pixel_size(size).build()
}

fn format_time_played(seconds: u64) -> String {
    if seconds == 0 { return String::new(); }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 { format!("{}h {}m played", hours, minutes) } else { format!("{}m played", minutes) }
}
