use crate::backend::instance::manager::Instance;
use adw::prelude::*;
use relm4::prelude::*;

#[derive(Debug)]
pub enum ContextMenuOutput {
    SelectInstance(usize),
    RenameInstance(usize),
    DeleteInstance(usize),
    MoveToGroupRequest(usize),
    RemoveFromGroup(usize),
    ChangeIcon(usize),
    RenameGroup(String),
    DeleteGroup(String),
    ShareInstance(usize),
}

/// Build a flat menu button without icon.
pub fn build_flat_menu_button(label: &str) -> gtk::Button {
    let btn = gtk::Button::builder()
        .has_frame(false)
        .css_classes(vec!["flat", "menu-btn"])
        .build();
    let lbl = gtk::Label::builder()
        .label(label)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .margin_start(8)
        .margin_end(8)
        .build();
    btn.set_child(Some(&lbl));
    btn
}

pub fn create_instance_popover(
    flat_idx: usize,
    _inst: &Instance,
    is_in_group: bool,
    sender: impl Fn(ContextMenuOutput) + 'static + Clone,
    parent: Option<&impl IsA<gtk::Widget>>,
) -> gtk::Popover {
    let popover = gtk::Popover::builder()
        .has_arrow(true)
        .autohide(true)
        .build();
    if let Some(p) = parent {
        popover.set_parent(p);
    }
    
    let menu_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(vec!["menu-box"])
        .width_request(160)
        .spacing(4)
        .build();

    // — Rename
    let rename_btn = build_flat_menu_button("Rename");
    {
        let s = sender.clone();
        let pop = popover.clone();
        rename_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::RenameInstance(flat_idx));
        });
    }
    menu_box.append(&rename_btn);

    // — Change Icon
    let icon_btn = build_flat_menu_button("Change Icon…");
    {
        let s = sender.clone();
        let pop = popover.clone();
        icon_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::ChangeIcon(flat_idx));
        });
    }
    menu_box.append(&icon_btn);

    // — Share
    let share_btn = build_flat_menu_button("Share…");
    {
        let s = sender.clone();
        let pop = popover.clone();
        share_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::ShareInstance(flat_idx));
        });
    }
    menu_box.append(&share_btn);

    menu_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // — Move to Group...
    let move_to_group_btn = build_flat_menu_button("Move to Group…");
    {
        let s = sender.clone();
        let pop = popover.clone();
        move_to_group_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::MoveToGroupRequest(flat_idx));
        });
    }
    menu_box.append(&move_to_group_btn);
    
    // — Remove from Group (only if in a group)
    if is_in_group {
        let remove_btn = build_flat_menu_button("Remove from Group");
        let s = sender.clone();
        let pop = popover.clone();
        remove_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::RemoveFromGroup(flat_idx));
        });
        menu_box.append(&remove_btn);
    }

    menu_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // — Delete
    let delete_btn = build_flat_menu_button("Delete");
    {
        let s = sender.clone();
        let pop = popover.clone();
        delete_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::DeleteInstance(flat_idx));
        });
    }
    menu_box.append(&delete_btn);

    popover.set_child(Some(&menu_box));
    popover
}

pub fn create_group_popover(
    group_name: String,
    sender: impl Fn(ContextMenuOutput) + 'static + Clone,
    parent: Option<&impl IsA<gtk::Widget>>,
) -> gtk::Popover {
    let popover = gtk::Popover::builder()
        .has_arrow(true)
        .autohide(true)
        .build();
    if let Some(p) = parent {
        popover.set_parent(p);
    }

    let menu_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .css_classes(vec!["menu-box"])
        .width_request(140)
        .spacing(4)
        .build();

    // Rename Group
    let rename_btn = build_flat_menu_button("Rename Group");
    {
        let s = sender.clone();
        let pop = popover.clone();
        let gn = group_name.clone();
        rename_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::RenameGroup(gn.clone()));
        });
    }
    menu_box.append(&rename_btn);

    menu_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    // Delete Group
    let delete_btn = build_flat_menu_button("Delete Group");
    {
        let s = sender.clone();
        let pop = popover.clone();
        let gn = group_name.clone();
        delete_btn.connect_clicked(move |_| {
            pop.popdown();
            s(ContextMenuOutput::DeleteGroup(gn.clone()));
        });
    }
    menu_box.append(&delete_btn);

    popover.set_child(Some(&menu_box));
    popover
}
