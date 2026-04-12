use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::Instance;
use crate::frontend::views::instance::helpers::{self, ContextMenuOutput};
use adw::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum SidebarInput {
    /// Full rebuild of sidebar contents.
    Rebuild(Vec<Instance>, InstanceGroups),
    /// Highlight the active instance row (flat index into Vec<Instance>).
    SetSelected(Option<usize>),
}

#[derive(Debug)]
pub enum SidebarOutput {
    /// User clicked the Overview row.
    ShowOverview,
    /// User clicked an instance row (flat index into Vec<Instance>).
    SelectInstance(usize),
    /// From context menu.
    RenameInstance(usize),
    /// From context menu.
    DeleteInstance(usize),
    /// Move instance (flat index) into a named group.
    MoveToGroup(usize, String),
    /// Request to show the move-to-group dialog.
    MoveToGroupRequest(usize),
    /// Remove instance from its current group.
    RemoveFromGroup(usize),
    /// Create a new group with the given name.
    CreateGroup(String),
    /// Rename group (old_name, new_name).
    RenameGroup(String, String),
    /// Delete group by name.
    DeleteGroup(String),
    /// Request to change instance icon.
    ChangeIcon(usize),
    /// Request to share the instance.
    ShareInstance(usize),
}

// ─── Model ────────────────────────────────────────────────────────────────────

pub struct SidebarList {
    main_box: gtk::Box,
    instances: Vec<Instance>,
    groups: InstanceGroups,
    selected: Option<usize>,
    overview_row: Option<adw::ActionRow>,
    instance_rows: HashMap<usize, adw::ActionRow>,
    all_lists: Vec<gtk::ListBox>,
    popovers: Vec<gtk::Popover>,
}

// ─── Component ────────────────────────────────────────────────────────────────

#[relm4::component(pub)]
impl SimpleComponent for SidebarList {
    type Init = ();
    type Input = SidebarInput;
    type Output = SidebarOutput;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_min_content_width: 240,
            set_vexpand: true,

            #[local_ref]
            main_box -> gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 18,
                set_margin_all: 12,
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 18);
        main_box.set_margin_all(12);

        let model = SidebarList {
            main_box: main_box.clone(),
            instances: Vec::new(),
            groups: InstanceGroups::default(),
            selected: None,
            overview_row: None,
            all_lists: Vec::new(),
            instance_rows: HashMap::new(),
            popovers: Vec::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: SidebarInput, sender: ComponentSender<Self>) {
        match msg {
            SidebarInput::Rebuild(instances, groups) => {
                self.instances = instances;
                self.groups = groups;
                self.rebuild_list(&sender);
            }
            SidebarInput::SetSelected(idx) => {
                self.selected = idx;
                self.update_selection();
            }
        }
    }
}

// ─── Builder helpers ──────────────────────────────────────────────────────────

impl SidebarList {
    /// Clears and rebuilds all rows in the ListBox.
    fn rebuild_list(&mut self, sender: &ComponentSender<Self>) {
        // Remove all existing children from main_box
        while let Some(child) = self.main_box.first_child() {
            self.main_box.remove(&child);
        }
        for pop in self.popovers.drain(..) {
            pop.unparent();
        }
        self.instance_rows.clear();
        self.all_lists.clear();

        // ── Overview row ─────────────────────────────────────────────────────
        let overview_row = adw::ActionRow::builder()
            .title("Overview")
            .activatable(true)
            .build();
        let home_icon = gtk::Image::from_icon_name("view-grid-symbolic");
        overview_row.add_prefix(&home_icon);

        {
            let s = sender.clone();
            overview_row.connect_activated(move |_| {
                s.output(SidebarOutput::ShowOverview).ok();
            });
        }
        self.overview_row = Some(overview_row.clone());
        
        let overview_list = gtk::ListBox::new();
        overview_list.add_css_class("boxed-list");
        overview_list.set_selection_mode(gtk::SelectionMode::Single);
        overview_list.append(&overview_row);
        self.main_box.append(&overview_list);
        self.all_lists.push(overview_list);

        // ── Grouped instances ────────────────────────────────────────────────
        let group_names_owned: Vec<String> = self.groups.sorted_group_names().iter().map(|s| s.to_string()).collect();

        for group_name in &group_names_owned {
            let info = match self.groups.groups.get(group_name.as_str()) {
                Some(i) => i,
                None => continue,
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

            let expander = adw::ExpanderRow::builder()
                .title(group_name.as_str())
                .show_enable_switch(false)
                .expanded(false)
                .selectable(false)
                .activatable(false)
                .build();

            let folder_icon = gtk::Image::from_icon_name("folder-symbolic");
            expander.add_prefix(&folder_icon);

            let popover = self.attach_group_context_menu(&expander, group_name.clone(), sender);
            self.popovers.push(popover);

            for (flat_idx, inst) in group_instances {
                let child_row = self.build_instance_row(flat_idx, inst, sender);
                self.instance_rows.insert(flat_idx, child_row.clone());
                expander.add_row(&child_row);
            }

            let group_list = gtk::ListBox::new();
            group_list.add_css_class("boxed-list");
            group_list.set_selection_mode(gtk::SelectionMode::None);
            group_list.append(&expander);
            self.main_box.append(&group_list);
            self.all_lists.push(group_list);

            if let Some(lb) = expander.first_child().and_then(|c| c.downcast::<gtk::ListBox>().ok()) {
                lb.set_selection_mode(gtk::SelectionMode::Single);
                lb.remove_css_class("boxed-list");
                self.all_lists.push(lb);
            }
        }

        // ── Ungrouped instances ──────────────────────────────────────────────
        let ungrouped: Vec<(usize, &Instance)> = self
            .instances
            .iter()
            .enumerate()
            .filter(|(_, inst)| {
                let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                self.groups.get_instance_group(folder).is_none()
            })
            .collect();

        if !ungrouped.is_empty() {
            let ungrouped_list = gtk::ListBox::new();
            ungrouped_list.add_css_class("boxed-list");
            ungrouped_list.set_selection_mode(gtk::SelectionMode::Single);
            for (flat_idx, inst) in ungrouped {
                let row = self.build_instance_row(flat_idx, inst, sender);
                self.instance_rows.insert(flat_idx, row.clone());
                ungrouped_list.append(&row);
            }
            self.main_box.append(&ungrouped_list);
            self.all_lists.push(ungrouped_list);
        }
    }

    /// Build a single instance `ActionRow` with all context-menu wiring.
    fn build_instance_row(
        &self,
        flat_idx: usize,
        inst: &Instance,
        sender: &ComponentSender<Self>,
    ) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(&inst.name)
            .activatable(true)
            .build();

        // Subtitle: MC version + loader
        let subtitle = match (&inst.minecraft_version, &inst.mod_loader) {
            (Some(v), Some(l)) => format!("{} • {}", v, l),
            (Some(v), None) => v.clone(),
            _ => String::new(),
        };
        if !subtitle.is_empty() {
            row.set_subtitle(&subtitle);
        }

        // Activate → select
        {
            let s = sender.clone();
            row.connect_activated(move |_| {
                s.output(SidebarOutput::SelectInstance(flat_idx)).ok();
            });
        }

        // Context menu button (…)
        let menu_btn = gtk::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .valign(gtk::Align::Center)
            .has_frame(false)
            .build();

        self.attach_instance_context_menu(&row, &menu_btn, flat_idx, inst, sender);
        row.add_suffix(&menu_btn);

        row
    }

    fn attach_instance_context_menu(&self, widget: &impl IsA<gtk::Widget>, menu_btn: &gtk::MenuButton, flat_idx: usize, inst: &Instance, sender: &ComponentSender<Self>) {
        let folder_name = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let s_clone = sender.clone();
        
        let popover = helpers::create_instance_popover(flat_idx, inst, self.groups.get_instance_group(&folder_name).is_some(), move |out| {
            match out {
                ContextMenuOutput::SelectInstance(idx) => s_clone.output(SidebarOutput::SelectInstance(idx)).ok(),
                ContextMenuOutput::RenameInstance(idx) => s_clone.output(SidebarOutput::RenameInstance(idx)).ok(),
                ContextMenuOutput::DeleteInstance(idx) => s_clone.output(SidebarOutput::DeleteInstance(idx)).ok(),
                ContextMenuOutput::MoveToGroupRequest(idx) => s_clone.output(SidebarOutput::MoveToGroupRequest(idx)).ok(),
                ContextMenuOutput::RemoveFromGroup(idx) => s_clone.output(SidebarOutput::RemoveFromGroup(idx)).ok(),
                ContextMenuOutput::ChangeIcon(idx) => s_clone.output(SidebarOutput::ChangeIcon(idx)).ok(),
                ContextMenuOutput::ShareInstance(idx) => s_clone.output(SidebarOutput::ShareInstance(idx)).ok(),
                _ => None,
            };
        }, None::<&gtk::Widget>);

        menu_btn.set_popover(Some(&popover));

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let p_reset = popover.clone();
            popover.connect_closed(move |_| {
                p_reset.set_pointing_to(None);
            });

            let pop = popover.clone();
            let w_ref = widget.clone();
            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                
                // Translate coordinates from widget (Row) to popover parent (MenuButton)
                if let Some(parent) = pop.parent() {
                    let mut tx = x as i32;
                    let mut ty = y as i32;
                    if let Some(p) = w_ref.compute_point(&parent, &gtk::graphene::Point::new(x as f32, y as f32)) {
                        tx = p.x() as i32;
                        ty = p.y() as i32;
                    }
                    pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(tx, ty, 1, 1)));
                }
                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
    }

    /// Context menu for an `adw::ExpanderRow` (group header).
    fn attach_group_context_menu(&self, widget: &impl IsA<gtk::Widget>, group_name: String, sender: &ComponentSender<Self>) -> gtk::Popover {
        let s_clone = sender.clone();
        let w_link = widget.clone();
        let popover = helpers::create_group_popover(group_name, move |out| {
            match out {
                ContextMenuOutput::RenameGroup(name) => s_clone.output(SidebarOutput::RenameGroup(name, "__rename__".to_string())).ok(),
                ContextMenuOutput::DeleteGroup(name) => s_clone.output(SidebarOutput::DeleteGroup(name)).ok(),
                _ => None,
            };
        }, Some(&w_link));

        let click_gesture = gtk::GestureClick::new();
        click_gesture.set_button(3); // Right click
        {
            let pop = popover.clone();
            click_gesture.connect_pressed(move |gesture, _, x, y| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                pop.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                pop.popup();
            });
        }
        widget.add_controller(click_gesture);
        popover
    }

    fn update_selection(&self) {
        // 1. Clear selection in ALL list boxes to prevent independent highlights
        for lb in &self.all_lists {
            lb.unselect_all();
        }

        // 2. Select the intended row
        if let Some(target) = self.selected {
            if let Some(row) = self.instance_rows.get(&target) {
                if let Some(lb) = row.parent().and_then(|p| p.downcast::<gtk::ListBox>().ok()) {
                    lb.select_row(Some(row));
                }
            }
        } else {
            // No instance selected → highlight overview
            if let Some(row) = &self.overview_row {
                if let Some(lb) = row.parent().and_then(|p| p.downcast::<gtk::ListBox>().ok()) {
                    lb.select_row(Some(row));
                }
            }
        }
    }
}

// ─── Legacy flat row (kept for public API compatibility) ──────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceAction {
    Select,
    Rename,
    Delete,
}
