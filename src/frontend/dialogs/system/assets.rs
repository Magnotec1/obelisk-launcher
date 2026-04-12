#![allow(unused_assignments)]
use crate::backend::download::assets::{
    delete_asset, format_size, scan_assets, scan_versions, AssetScanResult,
};
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AssetManagerInput {
    Open(PathBuf, Option<PathBuf>, Option<PathBuf>), // data_path, shared_path, instances_path
    Close,
    ScanComplete(AssetScanResult),
    ToggleExpand(usize),
    DeleteEntry(usize, usize, usize),
    DeleteConfirmed(usize, usize, usize),
    Refresh,
}

#[derive(Debug)]
pub enum AssetManagerOutput {}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

pub struct AssetManagerDialog {
    visible: bool,
    loading: bool,
    data_path: PathBuf,
    shared_path: Option<PathBuf>,
    instances_path: Option<PathBuf>,
    scan_result: Option<AssetScanResult>,
    list_box: gtk::ListBox,
    #[allow(dead_code)]
    window: Option<adw::Window>,
}

impl AssetManagerDialog {
    fn rebuild_list(&mut self) {
        // Remove all rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let scan = match &self.scan_result {
            Some(s) => s,
            None => return,
        };

        // -----------------------------------------------------------------------
        // "By Version" section at the top
        // -----------------------------------------------------------------------
        let version_groups = scan_versions(&self.data_path, self.shared_path.as_deref());
        if !version_groups.is_empty() {
            let ver_row = adw::ExpanderRow::new();
            ver_row.set_title("By Minecraft Version");
            ver_row.set_subtitle("Clean up all data for a specific version");

            let icon = gtk::Image::from_icon_name("package-x-generic-symbolic");
            ver_row.add_prefix(&icon);

            for group in &version_groups {
                let grp_row = adw::ExpanderRow::new();
                grp_row.set_title(&group.name);
                grp_row.set_subtitle(&format!(
                    "{} — client JAR + metadata + asset index",
                    format_size(group.total_size)
                ));
                grp_row.set_margin_start(12);

                // Group-level delete: deletes all entries in this version
                let all_paths: Vec<PathBuf> = group
                    .entries
                    .iter()
                    .map(|e| e.path.clone())
                    .filter(|p| !p.as_os_str().is_empty())
                    .collect();
                let grp_size: u64 = group
                    .entries
                    .iter()
                    .filter(|e| !e.path.as_os_str().is_empty())
                    .map(|e| e.size)
                    .sum();
                let grp_name = group.name.clone();
                let grp_del_btn = gtk::Button::new();
                grp_del_btn.set_icon_name("user-trash-symbolic");
                grp_del_btn.set_css_classes(&["flat", "circular", "color-destructive"]);
                grp_del_btn.set_tooltip_text(Some(&format!("Delete all {} data", grp_name)));
                grp_del_btn.set_valign(gtk::Align::Center);

                let grp_row_ref = grp_row.clone();
                grp_del_btn.connect_clicked(move |b| {
                    let dialog = adw::AlertDialog::new(
                        Some(&format!("Delete Minecraft {} Data?", grp_name)),
                        Some(&format!(
                            "This will delete the client JAR, version metadata, and asset index for Minecraft {}.\n\nTotal freed: approximately {}.\n\nGame asset objects are shared between versions and will not be deleted.\n\nThis action cannot be undone.",
                            grp_name,
                            format_size(grp_size)
                        )),
                    );
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("delete", "Delete Version Data");
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                    dialog.set_default_response(Some("cancel"));
                    dialog.set_close_response("cancel");

                    let paths: Vec<PathBuf> = all_paths.iter().filter(|p| !p.as_os_str().is_empty()).cloned().collect();
                    let row = grp_row_ref.clone();
                    dialog.connect_response(None, move |_dlg, response| {
                        if response == "delete" {
                            for path in &paths {
                                match delete_asset(path) {
                                    Ok(freed) => println!("Freed {} from {:?}", format_size(freed), path),
                                    Err(e) => eprintln!("Delete error: {}", e),
                                }
                            }
                            // Remove this group row from its parent
                            if let Some(parent) = row.parent() {
                                if let Some(parent_row) = parent.downcast_ref::<adw::ExpanderRow>() {
                                    parent_row.remove(&row);
                                }
                            }
                        }
                    });

                    if let Some(win) = b.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                        dialog.present(Some(&win));
                    }
                });

                let grp_suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                grp_suffix.set_valign(gtk::Align::Center);
                let grp_size_label = gtk::Label::new(Some(&format_size(group.total_size)));
                grp_size_label.set_css_classes(&["dim-label", "numeric"]);
                grp_suffix.append(&grp_size_label);
                grp_suffix.append(&grp_del_btn);
                grp_row.add_suffix(&grp_suffix);

                // Individual entries within the version
                for entry in &group.entries {
                    let ent_row = adw::ActionRow::new();
                    ent_row.set_title(&entry.name);
                    ent_row.set_margin_start(24);

                    let ent_size = gtk::Label::new(Some(&format_size(entry.size)));
                    ent_size.set_css_classes(&["dim-label", "numeric"]);

                    let entry_path = entry.path.clone();
                    let entry_name = entry.name.clone();
                    let entry_size = entry.size;

                    let del_btn = gtk::Button::new();
                    del_btn.set_icon_name("user-trash-symbolic");
                    del_btn.set_css_classes(&["flat", "circular"]);
                    del_btn.set_tooltip_text(Some("Delete"));
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.set_visible(!entry_path.as_os_str().is_empty());

                    let ent_row_ref = ent_row.clone();
                    del_btn.connect_clicked(move |b| {
                        let dialog = adw::AlertDialog::new(
                            Some("Delete Data?"),
                            Some(&format!(
                                "Delete \"{}\"?\n\nThis will free approximately {}.\n\nThis action cannot be undone.",
                                entry_name,
                                format_size(entry_size)
                            )),
                        );
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("delete", "Delete");
                        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");

                        let path = entry_path.clone();
                        let row = ent_row_ref.clone();
                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                match delete_asset(&path) {
                                    Ok(freed) => println!("Freed {}", format_size(freed)),
                                    Err(e) => eprintln!("Delete error: {}", e),
                                }
                                if let Some(parent) = row.parent() {
                                    if let Some(er) = parent.downcast_ref::<adw::ExpanderRow>() {
                                        er.remove(&row);
                                    }
                                }
                            }
                        });

                        if let Some(win) = b.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                            dialog.present(Some(&win));
                        }
                    });

                    let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    suffix.set_valign(gtk::Align::Center);
                    suffix.append(&ent_size);
                    suffix.append(&del_btn);
                    ent_row.add_suffix(&suffix);
                    grp_row.add_row(&ent_row);
                }

                ver_row.add_row(&grp_row);
            }

            self.list_box.append(&ver_row);
        }

        // -----------------------------------------------------------------------
        // Regular categories
        // -----------------------------------------------------------------------
        for category in scan.categories.iter() {
            let cat_row = adw::ExpanderRow::new();
            cat_row.set_title(&category.name);
            cat_row.set_subtitle(&format_size(category.total_size));

            let icon = gtk::Image::from_icon_name(category.icon);
            cat_row.add_prefix(&icon);

            let size_label = gtk::Label::new(Some(&format_size(category.total_size)));
            size_label.set_css_classes(&["dim-label", "numeric"]);
            cat_row.add_suffix(&size_label);

            for group in category.groups.iter() {
                let grp_row = adw::ExpanderRow::new();
                grp_row.set_title(&group.name);
                grp_row.set_subtitle(&format_size(group.total_size));
                grp_row.set_margin_start(12);

                let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                suffix.set_valign(gtk::Align::Center);

                let grp_size_label = gtk::Label::new(Some(&format_size(group.total_size)));
                grp_size_label.set_css_classes(&["dim-label", "numeric"]);
                suffix.append(&grp_size_label);

                // Add delete button for all groups EXCEPT "Instance Data"
                if category.name != "Instance Data" {
                    let grp_del_btn = gtk::Button::new();
                    grp_del_btn.set_icon_name("user-trash-symbolic");
                    grp_del_btn.set_css_classes(&["flat", "circular", "color-destructive"]);
                    grp_del_btn.set_tooltip_text(Some(&format!("Delete all in {}", group.name)));
                    grp_del_btn.set_valign(gtk::Align::Center);

                    let all_paths: Vec<std::path::PathBuf> = group
                        .entries
                        .iter()
                        .map(|e| e.path.clone())
                        .filter(|p| !p.as_os_str().is_empty())
                        .collect();
                    let grp_size: u64 = group
                        .entries
                        .iter()
                        .filter(|e| !e.path.as_os_str().is_empty())
                        .map(|e| e.size)
                        .sum();
                    let grp_name = group.name.clone();
                    let grp_row_ref = grp_row.clone();
                    let is_game_assets = category.name == "Game Assets";

                    grp_del_btn.connect_clicked(move |b| {
                        let extra_info = if is_game_assets {
                            "\n\nNote: Game asset objects are shared and will not be deleted; only the index file will be removed."
                        } else {
                            "\n\nThese are renewable asset files and can be redownloaded if needed."
                        };

                        let dialog = adw::AlertDialog::new(
                            Some(&format!("Delete all files in \"{}\"?", grp_name)),
                            Some(&format!(
                                "This will delete all files in this group.\n\nTotal freed: approximately {}.{}{}",
                                format_size(grp_size),
                                extra_info,
                                "\n\nThis action cannot be undone."
                            )),
                        );
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("delete", "Delete All");
                        dialog.set_response_appearance(
                            "delete",
                            adw::ResponseAppearance::Destructive,
                        );
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");

                        let paths = all_paths.clone();
                        let row = grp_row_ref.clone();
                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                for path in &paths {
                                    match delete_asset(path) {
                                        Ok(freed) => {
                                            println!("Freed {} from {:?}", format_size(freed), path)
                                        }
                                        Err(e) => eprintln!("Delete error: {}", e),
                                    }
                                }
                                if let Some(parent) = row.parent() {
                                    if let Some(parent_row) =
                                        parent.downcast_ref::<adw::ExpanderRow>()
                                    {
                                        parent_row.remove(&row);
                                    }
                                }
                            }
                        });

                        if let Some(win) = b.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                            dialog.present(Some(&win));
                        }
                    });
                    suffix.append(&grp_del_btn);
                }

                grp_row.add_suffix(&suffix);

                for entry in group.entries.iter() {
                    let ent_row = adw::ActionRow::new();
                    ent_row.set_title(&entry.name);
                    ent_row.set_subtitle(&format_size(entry.size));
                    ent_row.set_margin_start(24);

                    let ent_size = gtk::Label::new(Some(&format_size(entry.size)));
                    ent_size.set_css_classes(&["dim-label", "numeric"]);

                    let entry_path = entry.path.clone();
                    let entry_name = entry.name.clone();
                    let entry_size = entry.size;

                    let del_btn = gtk::Button::new();
                    del_btn.set_icon_name("user-trash-symbolic");
                    del_btn.set_css_classes(&["flat", "circular"]);
                    del_btn.set_tooltip_text(Some("Delete"));
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.set_visible(!entry_path.as_os_str().is_empty());

                    let ent_row_ref = ent_row.clone();
                    del_btn.connect_clicked(move |b| {
                        let dialog = adw::AlertDialog::new(
                            Some("Delete Data?"),
                            Some(&format!(
                                "Delete \"{}\"?\n\nThis will free approximately {}.\n\nThis action cannot be undone.",
                                entry_name,
                                format_size(entry_size)
                            )),
                        );
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("delete", "Delete");
                        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");

                        let path = entry_path.clone();
                        let row = ent_row_ref.clone();
                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                match delete_asset(&path) {
                                    Ok(freed) => println!("Freed {}", format_size(freed)),
                                    Err(e) => eprintln!("Delete error: {}", e),
                                }
                                if let Some(parent) = row.parent() {
                                    if let Some(er) = parent.downcast_ref::<adw::ExpanderRow>() {
                                        er.remove(&row);
                                    }
                                }
                            }
                        });

                        if let Some(win) = b.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                            dialog.present(Some(&win));
                        }
                    });

                    let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    suffix.set_valign(gtk::Align::Center);
                    suffix.append(&ent_size);
                    suffix.append(&del_btn);
                    ent_row.add_suffix(&suffix);
                    grp_row.add_row(&ent_row);
                }

                cat_row.add_row(&grp_row);
            }

            self.list_box.append(&cat_row);
        }
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[relm4::component(pub)]
impl SimpleComponent for AssetManagerDialog {
    type Init = ();
    type Input = AssetManagerInput;
    type Output = AssetManagerOutput;

    view! {
        adw::Window {
            set_title: Some("Asset Manager"),
            set_default_width: 720,
            set_default_height: 600,
            set_modal: true,
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(AssetManagerInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Asset Manager",
                        #[watch]
                        set_subtitle: &model.scan_result.as_ref()
                            .map(|s| format!("Total: {}", format_size(s.total_size)))
                            .unwrap_or_default(),
                    },

                    pack_end = &gtk::Button {
                        set_icon_name: "view-refresh-symbolic",
                        set_tooltip_text: Some("Refresh"),
                        connect_clicked => AssetManagerInput::Refresh,
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_vexpand: true,
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    #[watch]
                    set_visible_child_name: if model.loading { "loading" } else { "content" },

                    add_named[Some("loading")] = &adw::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_width_request: 32,
                        set_height_request: 32,
                    },

                    add_named[Some("content")] = &gtk::ScrolledWindow {
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_vexpand: true,

                        adw::Clamp {
                            set_maximum_size: 660,
                            set_tightening_threshold: 400,

                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 16,
                                set_spacing: 10,

                                // Summary / info card
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_css_classes: &["card"],
                                    set_margin_start: 2,
                                    set_margin_end: 2,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_margin_start: 16,
                                        set_margin_end: 16,
                                        set_margin_top: 16,
                                        set_margin_bottom: 8,
                                        set_spacing: 16,

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_hexpand: true,
                                            set_spacing: 2,

                                            gtk::Label {
                                                set_label: "Total Disk Usage",
                                                set_css_classes: &["heading"],
                                                set_halign: gtk::Align::Start,
                                            },
                                            gtk::Label {
                                                #[watch]
                                                set_label: &model.scan_result.as_ref()
                                                    .map(|s| format_size(s.total_size))
                                                    .unwrap_or_else(|| "Scanning…".to_string()),
                                                set_css_classes: &["title-1"],
                                                set_halign: gtk::Align::Start,
                                            },
                                        },
                                    },

                                    gtk::Separator {},

                                    // Data directory row
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 8,
                                        set_margin_start: 16,
                                        set_margin_end: 16,
                                        set_margin_top: 10,
                                        set_margin_bottom: 10,

                                        gtk::Image {
                                            set_icon_name: Some("folder-symbolic"),
                                            set_css_classes: &["dim-label"],
                                        },
                                        gtk::Label {
                                            set_label: "Data directory",
                                            set_css_classes: &["dim-label"],
                                        },
                                        gtk::Label {
                                            #[watch]
                                            set_label: &model.data_path.to_string_lossy(),
                                            set_css_classes: &["caption", "monospace"],
                                            set_halign: gtk::Align::Start,
                                            set_hexpand: true,
                                            set_ellipsize: gtk::pango::EllipsizeMode::Start,
                                            set_selectable: true,
                                        },
                                    },

                                    // Shared directory row (optional)
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 8,
                                        set_margin_start: 16,
                                        set_margin_end: 16,
                                        set_margin_top: 0,
                                        set_margin_bottom: 10,
                                        #[watch]
                                        set_visible: model.shared_path.is_some(),

                                        gtk::Image {
                                            set_icon_name: Some("folder-remote-symbolic"),
                                            set_css_classes: &["dim-label"],
                                        },
                                        gtk::Label {
                                            set_label: "Shared directory",
                                            set_css_classes: &["dim-label"],
                                        },
                                        gtk::Label {
                                            #[watch]
                                            set_label: &model.shared_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
                                            set_css_classes: &["caption", "monospace"],
                                            set_halign: gtk::Align::Start,
                                            set_hexpand: true,
                                            set_ellipsize: gtk::pango::EllipsizeMode::Start,
                                            set_selectable: true,
                                        },
                                    },
                                },

                                #[local_ref]
                                list_box_ref -> gtk::ListBox {
                                    set_css_classes: &["boxed-list"],
                                    set_selection_mode: gtk::SelectionMode::None,
                                },
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_box = gtk::ListBox::new();
        list_box.set_css_classes(&["boxed-list"]);
        list_box.set_selection_mode(gtk::SelectionMode::None);

        let model = AssetManagerDialog {
            visible: false,
            loading: false,
            data_path: PathBuf::new(),
            shared_path: None,
            instances_path: None,
            scan_result: None,
            list_box: list_box.clone(),
            window: Some(root.clone()),
        };

        let list_box_ref = &model.list_box;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AssetManagerInput::Open(data_path, shared_path, instances_path) => {
                self.visible = true;
                self.loading = true;
                self.data_path = data_path;
                self.shared_path = shared_path;
                self.instances_path = instances_path;

                let sender_clone = sender.input_sender().clone();
                let dp = self.data_path.clone();
                let sp = self.shared_path.clone();
                let ip = self.instances_path.clone();
                std::thread::spawn(move || {
                    let result = scan_assets(&dp, sp.as_deref(), ip.as_deref());
                    let _ = sender_clone.send(AssetManagerInput::ScanComplete(result));
                });
            }
            AssetManagerInput::Close => {
                self.visible = false;
            }
            AssetManagerInput::ScanComplete(result) => {
                self.scan_result = Some(result);
                self.loading = false;
                self.rebuild_list();
            }
            AssetManagerInput::ToggleExpand(_) => {}
            AssetManagerInput::DeleteEntry(_, _, _) => {}
            AssetManagerInput::DeleteConfirmed(_, _, _) => {}
            AssetManagerInput::Refresh => {
                self.loading = true;
                while let Some(child) = self.list_box.first_child() {
                    self.list_box.remove(&child);
                }
                self.scan_result = None;

                let sender_clone = sender.input_sender().clone();
                let dp = self.data_path.clone();
                let sp = self.shared_path.clone();
                let ip = self.instances_path.clone();
                std::thread::spawn(move || {
                    let result = scan_assets(&dp, sp.as_deref(), ip.as_deref());
                    let _ = sender_clone.send(AssetManagerInput::ScanComplete(result));
                });
            }
        }
    }
}
