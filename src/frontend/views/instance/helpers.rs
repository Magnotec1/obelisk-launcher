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
    ChangeIconFromFile(usize),
    ApplyDefaultIcon(usize),
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
    let menu = gtk::gio::Menu::new();
    
    // Main Section
    let main_section = gtk::gio::Menu::new();
    main_section.append(Some("Rename"), Some("pop.rename"));
    menu.append_section(None, &main_section);
    
    // Icon Submenu
    let icon_menu = gtk::gio::Menu::new();
    icon_menu.append(Some("Choose from File…"), Some("pop.change_icon_file"));
    icon_menu.append(Some("Use Default Icon"), Some("pop.change_icon_default"));
    main_section.append_submenu(Some("Change Icon"), &icon_menu);
    
    main_section.append(Some("Share…"), Some("pop.share"));
    
    // Group Section
    let group_section = gtk::gio::Menu::new();
    group_section.append(Some("Move to Group…"), Some("pop.move_group"));
    if is_in_group {
        group_section.append(Some("Remove from Group"), Some("pop.remove_group"));
    }
    menu.append_section(None, &group_section);
    
    // Delete Section
    let delete_section = gtk::gio::Menu::new();
    delete_section.append(Some("Delete"), Some("pop.delete"));
    menu.append_section(None, &delete_section);

    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    popover.set_has_arrow(true);
    
    if let Some(p) = parent {
        popover.set_parent(p);
    }
    
    let action_group = gtk::gio::SimpleActionGroup::new();
    
    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("rename", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::RenameInstance(flat_idx)));
    action_group.add_action(&act);
    
    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("change_icon_file", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::ChangeIconFromFile(flat_idx)));
    action_group.add_action(&act);

    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("change_icon_default", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::ApplyDefaultIcon(flat_idx)));
    action_group.add_action(&act);

    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("share", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::ShareInstance(flat_idx)));
    action_group.add_action(&act);

    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("move_group", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::MoveToGroupRequest(flat_idx)));
    action_group.add_action(&act);

    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("remove_group", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::RemoveFromGroup(flat_idx)));
    action_group.add_action(&act);

    let s = sender.clone();
    let act = gtk::gio::SimpleAction::new("delete", None);
    act.connect_activate(move |_, _| s(ContextMenuOutput::DeleteInstance(flat_idx)));
    action_group.add_action(&act);

    popover.insert_action_group("pop", Some(&action_group));
    popover.upcast::<gtk::Popover>()
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
