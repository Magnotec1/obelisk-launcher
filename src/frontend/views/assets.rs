use crate::backend::download::assets::{
    delete_asset, format_size, scan_versions, AssetScanResult,
};
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AssetInput {
    UpdateData(AssetScanResult, PathBuf, Option<PathBuf>, Option<PathBuf>),
    Refresh,
    Loading(bool),
    Delete(PathBuf),
    SetPage(u32),
}

#[derive(Debug)]
pub enum AssetOutput {
    RefreshRequest,
}

pub struct AssetManagerView {
    loading: bool,
    data_path: PathBuf,
    shared_path: Option<PathBuf>,
    instances_path: Option<PathBuf>,
    scan_result: Option<AssetScanResult>,
    view_stack: adw::ViewStack,
    dropdown_model: gtk::StringList,
    page_ids: Vec<String>,
    sender: Option<ComponentSender<AssetManagerView>>,
}

impl AssetManagerView {
    fn rebuild_list(&mut self) {
        let _sender = match &self.sender {
            Some(s) => s.clone(),
            None => return,
        };

        // Remove all pages
        while let Some(child) = self.view_stack.first_child() {
            self.view_stack.remove(&child);
        }
        self.dropdown_model.splice(0, self.dropdown_model.n_items(), &[]);
        self.page_ids.clear();

        let scan = match &self.scan_result {
            Some(s) => s,
            None => return,
        };

        use std::rc::Rc;
        use std::cell::RefCell;
        use std::collections::HashMap;

        let path_widgets: Rc<RefCell<HashMap<PathBuf, Vec<gtk::Widget>>>> = Rc::new(RefCell::new(HashMap::new()));

        let create_page_box = || -> gtk::Box {
            let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
            content_box.set_margin_top(16);
            content_box
        };

        // -----------------------------------------------------------------------
        // "By Version" section at the top
        // -----------------------------------------------------------------------
        let version_groups = scan_versions(&self.data_path, self.shared_path.as_deref());
        if !version_groups.is_empty() {
            let content_box = create_page_box();

            let ver_group = adw::PreferencesGroup::new();
            ver_group.set_title("By Minecraft Version");
            ver_group.set_description(Some("Clean up all data for a specific version"));

            for group in &version_groups {
                let grp_row = adw::ExpanderRow::new();
                grp_row.set_title(&group.name);
                grp_row.set_subtitle(&format!(
                    "{} — client JAR + metadata + asset index",
                    format_size(group.total_size)
                ));

                // Group-level delete
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

                let widgets_map = path_widgets.clone();
                let p_group = ver_group.clone();
                let g_row = grp_row.clone();

                grp_del_btn.connect_clicked(move |b| {
                    let dialog = adw::AlertDialog::new(
                        Some(&format!("Delete Minecraft {} Data?", grp_name)),
                        Some(&format!(
                            "This will delete the client JAR, version metadata, and asset index for Minecraft {}.\n\nTotal freed: approximately {}.\n\nThis action cannot be undone.",
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
                    let w_map = widgets_map.clone();
                    let parent_group = p_group.clone();
                    let row_to_remove = g_row.clone();

                    dialog.connect_response(None, move |_dlg, response| {
                        if response == "delete" {
                            for path in &paths {
                                let _ = delete_asset(path);
                                if let Some(widgets) = w_map.borrow_mut().remove(path) {
                                    for widget in widgets {
                                        if let Some(parent) = widget.parent() {
                                            if let Ok(list_box) = parent.downcast::<gtk::ListBox>() {
                                                list_box.remove(&widget);
                                            }
                                        }
                                    }
                                }
                            }
                            parent_group.remove(&row_to_remove);
                        }
                    });

                    if let Some(win) = b.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                        dialog.present(Some(&win));
                    }
                });

                let grp_suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                grp_suffix.set_valign(gtk::Align::Center);
                grp_suffix.append(&grp_del_btn);
                grp_row.add_suffix(&grp_suffix);

                for entry in &group.entries {
                    let ent_row = adw::ActionRow::new();
                    ent_row.set_title(&entry.name);
                    ent_row.set_subtitle(&format_size(entry.size));
                    ent_row.add_css_class("nested");

                    let entry_path = entry.path.clone();
                    let entry_name = entry.name.clone();
                    let entry_size = entry.size;

                    let widget_ref = ent_row.clone().upcast::<gtk::Widget>();
                    path_widgets.borrow_mut().entry(entry_path.clone()).or_default().push(widget_ref);

                    let del_btn = gtk::Button::new();
                    del_btn.set_icon_name("user-trash-symbolic");
                    del_btn.set_css_classes(&["flat", "circular"]);
                    del_btn.set_tooltip_text(Some("Delete"));
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.set_visible(!entry_path.as_os_str().is_empty());

                    let w_map = path_widgets.clone();
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
                        let w_map_clone = w_map.clone();
                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                let _ = delete_asset(&path);
                                if let Some(widgets) = w_map_clone.borrow_mut().remove(&path) {
                                    for widget in widgets {
                                        if let Some(parent) = widget.parent() {
                                            if let Ok(list_box) = parent.downcast::<gtk::ListBox>() {
                                                list_box.remove(&widget);
                                            }
                                        }
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
                    suffix.append(&del_btn);
                    ent_row.add_suffix(&suffix);
                    grp_row.add_row(&ent_row);
                }

                ver_group.add(&grp_row);
            }

            content_box.append(&ver_group);
            self.view_stack.add_titled(&content_box, Some("versions"), "Versions");
            
            self.dropdown_model.append("Versions");
            self.page_ids.push("versions".to_string());
        }

        // Categories
        for category in scan.categories.iter() {
            let content_box = create_page_box();

            let cat_group = adw::PreferencesGroup::new();
            cat_group.set_title(&category.name);
            cat_group.set_description(Some(&format_size(category.total_size)));

            for group in category.groups.iter() {
                let grp_row = adw::ExpanderRow::new();
                grp_row.set_title(&group.name);
                grp_row.set_subtitle(&format_size(group.total_size));

                let suffix = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                suffix.set_valign(gtk::Align::Center);

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
                    let is_game_assets = category.name == "Game Assets";
                    
                    let widgets_map = path_widgets.clone();
                    let p_group = cat_group.clone();
                    let g_row = grp_row.clone();

                    grp_del_btn.connect_clicked(move |b| {
                        let extra_info = if is_game_assets {
                            "\n\nNote: Game asset objects are shared and will not be deleted; only the index file will be removed."
                        } else {
                            "\n\nThese are renewable asset files and can be redownloaded if needed."
                        };

                        let dialog = adw::AlertDialog::new(
                            Some(&format!("Delete all files in \"{}\"?", grp_name)),
                            Some(&format!(
                                "This will delete all files in this group.\n\nTotal freed: approximately {}.{}{}\n\nThis action cannot be undone.",
                                format_size(grp_size),
                                extra_info,
                                ""
                            )),
                        );
                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("delete", "Delete All");
                        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");

                        let paths = all_paths.clone();
                        let w_map = widgets_map.clone();
                        let parent_group = p_group.clone();
                        let row_to_remove = g_row.clone();

                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                for path in &paths {
                                    let _ = delete_asset(path);
                                    if let Some(widgets) = w_map.borrow_mut().remove(path) {
                                        for widget in widgets {
                                            if let Some(parent) = widget.parent() {
                                                if let Ok(list_box) = parent.downcast::<gtk::ListBox>() {
                                                    list_box.remove(&widget);
                                                }
                                            }
                                        }
                                    }
                                }
                                parent_group.remove(&row_to_remove);
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
                    ent_row.add_css_class("nested");

                    let entry_path = entry.path.clone();
                    let entry_name = entry.name.clone();
                    let entry_size = entry.size;

                    let widget_ref = ent_row.clone().upcast::<gtk::Widget>();
                    path_widgets.borrow_mut().entry(entry_path.clone()).or_default().push(widget_ref);

                    let del_btn = gtk::Button::new();
                    del_btn.set_icon_name("user-trash-symbolic");
                    del_btn.set_css_classes(&["flat", "circular"]);
                    del_btn.set_tooltip_text(Some("Delete"));
                    del_btn.set_valign(gtk::Align::Center);
                    del_btn.set_visible(!entry_path.as_os_str().is_empty());

                    let w_map = path_widgets.clone();
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
                        let w_map_clone = w_map.clone();
                        dialog.connect_response(None, move |_dlg, response| {
                            if response == "delete" {
                                let _ = delete_asset(&path);
                                if let Some(widgets) = w_map_clone.borrow_mut().remove(&path) {
                                    for widget in widgets {
                                        if let Some(parent) = widget.parent() {
                                            if let Ok(list_box) = parent.downcast::<gtk::ListBox>() {
                                                list_box.remove(&widget);
                                            }
                                        }
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
                    suffix.append(&del_btn);
                    ent_row.add_suffix(&suffix);
                    grp_row.add_row(&ent_row);
                }

                cat_group.add(&grp_row);
            }

            content_box.append(&cat_group);
            let id = category.name.to_lowercase().replace(" ", "-");
            let page = self.view_stack.add_titled(&content_box, Some(&id), &category.name);
            page.set_icon_name(Some("folder-symbolic"));
            
            self.dropdown_model.append(&category.name);
            self.page_ids.push(id);
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AssetManagerView {
    type Init = ();
    type Input = AssetInput;
    type Output = AssetOutput;

    view! {
        adw::Bin {
            gtk::Stack {
                set_vexpand: true,

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
                        set_label: "Scanning...",
                        set_css_classes: &["dim-label"],
                    }
                },

                add_named[Some("content")] = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vexpand: true,

                    #[wrap(Some)]
                    set_child = &adw::Clamp {
                        set_maximum_size: 600,
                        set_tightening_threshold: 400,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                            set_margin_all: 16,

                            // Summary / info card
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_css_classes: &["card"],

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_margin_all: 16,
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
                                    set_margin_top: 16,
                                    set_margin_bottom: 16,

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
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 8,
                                    set_margin_start: 16,
                                    set_margin_end: 16,
                                    set_margin_bottom: 16,

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
                                        set_label: &model.shared_path.as_ref().map_or_else(|| "None".to_string(), |p| p.to_string_lossy().to_string()),
                                        set_css_classes: &["caption", "monospace"],
                                        set_halign: gtk::Align::Start,
                                        set_hexpand: true,
                                        set_ellipsize: gtk::pango::EllipsizeMode::Start,
                                        set_selectable: true,
                                    },
                                }
                            },

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Categories",
                                    set_css_classes: &["heading"],
                                    set_halign: gtk::Align::Start,
                                    set_margin_start: 4,
                                },
                                gtk::DropDown {
                                    set_model: Some(&model.dropdown_model),
                                    set_halign: gtk::Align::Start,
                                    connect_selected_notify[sender] => move |dropdown| {
                                        sender.input(AssetInput::SetPage(dropdown.selected()));
                                    }
                                }
                            },

                            #[local_ref]
                            view_stack_ref -> adw::ViewStack {
                                set_vexpand: true,
                            }
                        }
                    }
                },

                // Must come after add_named so children exist on first render
                #[watch]
                set_visible_child_name: if model.loading { "loading" } else { "content" },
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let view_stack = adw::ViewStack::new();

        let model = AssetManagerView {
            loading: true,
            data_path: PathBuf::new(),
            shared_path: None,
            instances_path: None,
            scan_result: None,
            view_stack: view_stack.clone(),
            dropdown_model: gtk::StringList::new(&[]),
            page_ids: Vec::new(),
            sender: Some(sender.clone()),
        };

        let view_stack_ref = &model.view_stack;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AssetInput::UpdateData(result, data_path, shared_path, instances_path) => {
                self.scan_result = Some(result);
                self.data_path = data_path;
                self.shared_path = shared_path;
                self.instances_path = instances_path;
                self.loading = false;
                self.rebuild_list();
            }
            AssetInput::Refresh => {
                _sender.output(AssetOutput::RefreshRequest).ok();
            }
            AssetInput::Loading(loading) => {
                self.loading = loading;
                if loading {
                    while let Some(page) = self.view_stack.pages().item(0).and_downcast::<adw::ViewStackPage>() {
                        self.view_stack.remove(&page.child());
                    }
                }
            }
            AssetInput::Delete(path) => {
                let _ = delete_asset(&path);
                self.rebuild_list();
            }
            AssetInput::SetPage(index) => {
                if let Some(id) = self.page_ids.get(index as usize) {
                    self.view_stack.set_visible_child_name(id);
                }
            }
        }
    }
}
