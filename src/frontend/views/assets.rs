use crate::backend::download::assets::{delete_asset, format_size, scan_versions, AssetScanResult};
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

fn get_category_color_hex(name: &str) -> &'static str {
    match name {
        "Game Assets" => "#3584e4",
        "Client JARs" => "#f6d32d",
        "Libraries" => "#2ec27e",
        "Version Metadata" => "#813d9c",
        "Instance Data" => "#e66100",
        "Versions" => "#e01b24",
        _ => "#777777",
    }
}

fn hex_to_rgb(hex: &str) -> (f64, f64, f64) {
    if hex.len() < 7 {
        return (0.5, 0.5, 0.5);
    }
    let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(128) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(128) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(128) as f64 / 255.0;
    (r, g, b)
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AssetInput {
    UpdateData(AssetScanResult, PathBuf, Option<PathBuf>, Option<PathBuf>),
    Refresh,
    Loading(bool),
    Delete(PathBuf),
    ShowCategory(String, String),
    ShowCategoriesPage,
}

#[derive(Debug)]
pub enum AssetOutput {
    RefreshRequest,
    SubpageChanged(Option<String>),
}

pub struct AssetManagerView {
    loading: bool,
    data_path: PathBuf,
    shared_path: Option<PathBuf>,
    instances_path: Option<PathBuf>,
    scan_result: Option<AssetScanResult>,
    view_stack: adw::ViewStack,
    category_list_box: gtk::ListBox,
    subpage_stack: gtk::Stack,
    active_category_title: String,
    page_ids: Vec<String>,
    sender: Option<ComponentSender<AssetManagerView>>,

    chart_slices: std::rc::Rc<std::cell::RefCell<Vec<(String, f64, &'static str)>>>,
    chart_drawing_area: gtk::DrawingArea,
    legend_box: gtk::Box,
}

impl AssetManagerView {
    fn create_category_row(
        &self,
        title: &str,
        subtitle: &str,
        icon_name: &str,
        page_id: String,
    ) -> adw::ActionRow {
        let row = adw::ActionRow::new();
        row.set_title(title);
        row.set_subtitle(subtitle);
        row.set_activatable(true);

        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_css_classes(&["dim-label"]);
        row.add_prefix(&icon);

        let arrow = gtk::Image::from_icon_name("go-next-symbolic");
        arrow.set_css_classes(&["dim-label"]);
        row.add_suffix(&arrow);

        if let Some(ref sender) = self.sender {
            let sender_clone = sender.clone();
            let title_owned = title.to_string();
            row.connect_activated(move |_| {
                sender_clone.input(AssetInput::ShowCategory(
                    page_id.clone(),
                    title_owned.clone(),
                ));
            });
        }
        row
    }

    fn create_page_box(&self) -> gtk::Box {
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        content_box.set_margin_top(16);
        content_box
    }

    fn remove_widgets_for_paths(
        w_map: &std::rc::Rc<
            std::cell::RefCell<std::collections::HashMap<PathBuf, Vec<gtk::Widget>>>,
        >,
        paths: &[PathBuf],
    ) {
        let mut map = w_map.borrow_mut();
        for path in paths {
            if let Some(widgets) = map.remove(path) {
                for widget in widgets {
                    if let Some(parent) = widget.parent() {
                        if let Ok(list_box) = parent.downcast::<gtk::ListBox>() {
                            list_box.remove(&widget);
                        }
                    }
                }
            }
        }
    }

    fn build_versions_page(
        &self,
        version_groups: &[crate::backend::download::assets::AssetGroup],
        path_widgets: &std::rc::Rc<
            std::cell::RefCell<std::collections::HashMap<PathBuf, Vec<gtk::Widget>>>,
        >,
    ) -> gtk::Box {
        let content_box = self.create_page_box();

        let ver_group = adw::PreferencesGroup::new();
        ver_group.set_title("By Minecraft Version");
        ver_group.set_description(Some("Clean up all data for a specific version"));

        for group in version_groups {
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

            let w_map = path_widgets.clone();
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

                let paths = all_paths.clone();
                let w_map_clone = w_map.clone();
                let parent_group = p_group.clone();
                let row_to_remove = g_row.clone();

                dialog.connect_response(None, move |_dlg, response| {
                    if response == "delete" {
                        for path in &paths {
                            let _ = delete_asset(path);
                        }
                        Self::remove_widgets_for_paths(&w_map_clone, &paths);
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
                path_widgets
                    .borrow_mut()
                    .entry(entry_path.clone())
                    .or_default()
                    .push(widget_ref);

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
                            Self::remove_widgets_for_paths(&w_map_clone, &[path.clone()]);
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
        content_box
    }

    fn build_category_page(
        &self,
        category: &crate::backend::download::assets::AssetCategory,
        path_widgets: &std::rc::Rc<
            std::cell::RefCell<std::collections::HashMap<PathBuf, Vec<gtk::Widget>>>,
        >,
    ) -> gtk::Box {
        let content_box = self.create_page_box();

        let cat_group = adw::PreferencesGroup::new();
        cat_group.set_title(&category.name);
        cat_group.set_description(Some(&format_size(category.total_size)));

        for group in &category.groups {
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

                let w_map = path_widgets.clone();
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
                    let w_map_clone = w_map.clone();
                    let parent_group = p_group.clone();
                    let row_to_remove = g_row.clone();

                    dialog.connect_response(None, move |_dlg, response| {
                        if response == "delete" {
                            for path in &paths {
                                let _ = delete_asset(path);
                            }
                            Self::remove_widgets_for_paths(&w_map_clone, &paths);
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

            for entry in &group.entries {
                let ent_row = adw::ActionRow::new();
                ent_row.set_title(&entry.name);
                ent_row.set_subtitle(&format_size(entry.size));
                ent_row.add_css_class("nested");

                let entry_path = entry.path.clone();
                let entry_name = entry.name.clone();
                let entry_size = entry.size;

                let widget_ref = ent_row.clone().upcast::<gtk::Widget>();
                path_widgets
                    .borrow_mut()
                    .entry(entry_path.clone())
                    .or_default()
                    .push(widget_ref);

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
                            Self::remove_widgets_for_paths(&w_map_clone, &[path.clone()]);
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
        content_box
    }

    fn rebuild_list(&mut self) {
        let _sender = match &self.sender {
            Some(s) => s.clone(),
            None => return,
        };

        // Remove all pages
        while let Some(child) = self.view_stack.first_child() {
            self.view_stack.remove(&child);
        }
        while let Some(child) = self.category_list_box.first_child() {
            self.category_list_box.remove(&child);
        }
        while let Some(child) = self.legend_box.first_child() {
            self.legend_box.remove(&child);
        }
        self.page_ids.clear();

        let scan = match &self.scan_result {
            Some(s) => s,
            None => return,
        };

        // Rebuild slices and legend
        let mut slices = Vec::new();
        let version_groups = scan_versions(&self.data_path, self.shared_path.as_deref());
        let versions_size: u64 = if !version_groups.is_empty() {
            version_groups.iter().map(|g| g.total_size).sum()
        } else {
            0
        };
        let mut total: u64 = versions_size;
        for cat in &scan.categories {
            total += cat.total_size;
        }

        if total > 0 {
            if versions_size > 0 {
                slices.push((
                    "Versions".to_string(),
                    versions_size as f64 / total as f64,
                    get_category_color_hex("Versions"),
                ));
            }
            for cat in &scan.categories {
                if cat.total_size > 0 {
                    slices.push((
                        cat.name.clone(),
                        cat.total_size as f64 / total as f64,
                        get_category_color_hex(&cat.name),
                    ));
                }
            }
            slices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        }
        *self.chart_slices.borrow_mut() = slices.clone();

        for (name, percentage, hex) in &slices {
            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);

            let color_label = gtk::Label::builder()
                .use_markup(true)
                .label(format!("<span foreground='{}'>●</span>", hex))
                .build();

            let name_label = gtk::Label::new(Some(name));
            name_label.set_css_classes(&["caption"]);

            let pct_label = gtk::Label::new(Some(&format!("{:.1}%", percentage * 100.0)));
            pct_label.set_css_classes(&["caption", "dim-label"]);
            pct_label.set_hexpand(true);
            pct_label.set_halign(gtk::Align::End);

            row_box.append(&color_label);
            row_box.append(&name_label);
            row_box.append(&pct_label);

            self.legend_box.append(&row_box);
        }
        self.chart_drawing_area.queue_draw();

        use std::cell::RefCell;
        use std::collections::HashMap;
        use std::rc::Rc;

        let path_widgets: Rc<RefCell<HashMap<PathBuf, Vec<gtk::Widget>>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // 1. Build Versions page if there are versions
        let version_groups = scan_versions(&self.data_path, self.shared_path.as_deref());
        if !version_groups.is_empty() {
            let versions_total_size: u64 = version_groups.iter().map(|g| g.total_size).sum();
            let content_box = self.build_versions_page(&version_groups, &path_widgets);
            self.view_stack
                .add_titled(&content_box, Some("versions"), "Versions");

            let row = self.create_category_row(
                "Versions",
                &format_size(versions_total_size),
                "folder-symbolic",
                "versions".to_string(),
            );
            self.category_list_box.append(&row);
            self.page_ids.push("versions".to_string());
        }

        // 2. Build Category pages
        for category in &scan.categories {
            let content_box = self.build_category_page(category, &path_widgets);
            let id = category.name.to_lowercase().replace(" ", "-");
            let page = self
                .view_stack
                .add_titled(&content_box, Some(&id), &category.name);
            page.set_icon_name(Some("folder-symbolic"));

            let row = self.create_category_row(
                &category.name,
                &format_size(category.total_size),
                category.icon,
                id.clone(),
            );
            self.category_list_box.append(&row);
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

                            #[local_ref]
                            subpage_stack_ref -> gtk::Stack {
                                set_vexpand: true,
                                set_vhomogeneous: false,
                                set_hhomogeneous: false,
                                set_transition_type: gtk::StackTransitionType::SlideLeftRight,

                                add_named[Some("categories")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 16,

                                    // Summary / info card
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_css_classes: &["card"],

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_margin_all: 16,
                                            set_margin_bottom: 12,
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

                                    // Storage Breakdown card
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 24,
                                        set_css_classes: &["card"],
                                        set_margin_start: 0,
                                        set_margin_end: 0,
                                        set_margin_top: 0,

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_spacing: 20,
                                            set_margin_all: 16,
                                            set_hexpand: true,

                                            #[local_ref]
                                            chart_drawing_area_ref -> gtk::DrawingArea {
                                                set_halign: gtk::Align::Start,
                                                set_valign: gtk::Align::Center,
                                            },

                                            #[local_ref]
                                            legend_box_ref -> gtk::Box {
                                                set_hexpand: true,
                                                set_valign: gtk::Align::Center,
                                            }
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

                                        #[local_ref]
                                        category_list_box_ref -> gtk::ListBox {
                                            set_css_classes: &["boxed-list"],
                                            set_selection_mode: gtk::SelectionMode::None,
                                        }
                                    }
                                },

                                add_named[Some("details")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_vexpand: true,

                                    #[local_ref]
                                    view_stack_ref -> adw::ViewStack {
                                        set_vexpand: true,
                                    }
                                }
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
        view_stack.set_vhomogeneous(false);
        view_stack.set_hhomogeneous(false);

        let category_list_box = gtk::ListBox::new();
        category_list_box.set_css_classes(&["boxed-list"]);
        category_list_box.set_selection_mode(gtk::SelectionMode::None);

        let subpage_stack = gtk::Stack::new();
        subpage_stack.set_vhomogeneous(false);
        subpage_stack.set_hhomogeneous(false);

        let chart_drawing_area = gtk::DrawingArea::new();
        chart_drawing_area.set_content_width(120);
        chart_drawing_area.set_content_height(120);

        let legend_box = gtk::Box::new(gtk::Orientation::Vertical, 6);

        let model = AssetManagerView {
            loading: true,
            data_path: PathBuf::new(),
            shared_path: None,
            instances_path: None,
            scan_result: None,
            view_stack: view_stack.clone(),
            category_list_box: category_list_box.clone(),
            subpage_stack: subpage_stack.clone(),
            active_category_title: String::new(),
            page_ids: Vec::new(),
            sender: Some(sender.clone()),
            chart_slices: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            chart_drawing_area: chart_drawing_area.clone(),
            legend_box: legend_box.clone(),
        };

        let slices_clone = model.chart_slices.clone();
        chart_drawing_area.set_draw_func(move |_area, cr, width, height| {
            let slices = slices_clone.borrow();
            let cx = width as f64 / 2.0;
            let cy = height as f64 / 2.0;
            let radius = (width.min(height) as f64 / 2.0) - 6.0;
            let stroke_width = radius * 0.35;
            let middle_radius = radius - (stroke_width / 2.0);

            if radius <= 0.0 {
                return;
            }

            if slices.is_empty() {
                cr.set_source_rgba(0.7, 0.7, 0.7, 0.2);
                cr.set_line_width(stroke_width);
                cr.arc(cx, cy, middle_radius, 0.0, 2.0 * std::f64::consts::PI);
                let _ = cr.stroke();
                return;
            }

            let mut current_angle = -std::f64::consts::FRAC_PI_2;
            for (_name, percentage, hex) in slices.iter() {
                let angle = percentage * 2.0 * std::f64::consts::PI;
                let (r, g, b) = hex_to_rgb(hex);
                cr.set_source_rgb(r, g, b);
                cr.set_line_width(stroke_width);
                cr.arc(cx, cy, middle_radius, current_angle, current_angle + angle);
                let _ = cr.stroke();
                current_angle += angle;
            }
        });

        let view_stack_ref = &model.view_stack;
        let category_list_box_ref = &model.category_list_box;
        let subpage_stack_ref = &model.subpage_stack;
        let chart_drawing_area_ref = &model.chart_drawing_area;
        let legend_box_ref = &model.legend_box;
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
                    while let Some(page) = self
                        .view_stack
                        .pages()
                        .item(0)
                        .and_downcast::<adw::ViewStackPage>()
                    {
                        self.view_stack.remove(&page.child());
                    }
                    while let Some(child) = self.category_list_box.first_child() {
                        self.category_list_box.remove(&child);
                    }
                    while let Some(child) = self.legend_box.first_child() {
                        self.legend_box.remove(&child);
                    }
                    self.chart_slices.borrow_mut().clear();
                    self.subpage_stack.set_visible_child_name("categories");
                    _sender.output(AssetOutput::SubpageChanged(None)).ok();
                }
            }
            AssetInput::Delete(path) => {
                let _ = delete_asset(&path);
                self.rebuild_list();
            }
            AssetInput::ShowCategory(id, title) => {
                self.view_stack.set_visible_child_name(&id);
                self.active_category_title = title.clone();
                self.subpage_stack.set_visible_child_name("details");
                _sender
                    .output(AssetOutput::SubpageChanged(Some(title)))
                    .ok();
            }
            AssetInput::ShowCategoriesPage => {
                self.subpage_stack.set_visible_child_name("categories");
                _sender.output(AssetOutput::SubpageChanged(None)).ok();
            }
        }
    }
}
