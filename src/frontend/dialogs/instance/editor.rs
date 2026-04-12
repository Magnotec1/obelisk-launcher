#![allow(unused_assignments)]
use adw::prelude::*;
use gtk::gdk;
use relm4::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use crate::frontend::dialogs::external::download::{DownloadStatusBar, DownloadStatusBarInput};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EditorItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub filename: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub sources: Option<String>,
    pub icon_path: Option<String>,
    pub is_checked: bool,
    pub size: Option<String>,
    pub seed: Option<String>,
    pub last_played: Option<String>,
}

#[derive(Debug, Clone)]
pub enum EditorType {
    Mods,
    Components,
    ResourcePacks,
    ShaderPacks,
    Worlds,
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum EditorInput {
    Open(EditorType, String, Vec<EditorItem>),
    Close,

    // Selection
    FocusItem(usize),
    ToggleCheck(usize),
    SetChecked(usize, bool),
    SelectAll,
    DeselectAll,

    // Mode
    SetMultiSelect(bool),

    // Actions
    RemoveSelected,
    RemoveFocused,
    OpenFolder,
    BrowseModrinth,
    AddItemsRequest,
    AddItems(Vec<PathBuf>),
    ConfirmRemove(Vec<String>),

    // Search
    Search(String),

    // Keyboard
    KeyPressed(gdk::Key, gdk::ModifierType),

    // Drag & Drop
    FilesDropped(Vec<PathBuf>),

    // Status/Toasts
    UpdateItems(Vec<EditorItem>),
    DownloadProgress(String, f32, bool),
    ShowToast(String),
}

#[derive(Debug)]
pub enum EditorOutput {
    RemoveMods(Vec<String>),
    RemoveComponents(Vec<String>),
    RemoveResourcePacks(Vec<String>),
    RemoveShaderPacks(Vec<String>),
    RemoveWorlds(Vec<String>),
    AddItems(EditorType, Vec<PathBuf>),
    OpenFolder(EditorType),
    BrowseModrinth(EditorType),
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

pub struct InstanceEditorDialog {
    visible: bool,
    title: String,
    editor_type: EditorType,
    items: Vec<EditorItem>,
    focused_index: Option<usize>,
    multi_select: bool,
    search_query: String,

    // Maps list-box visible row index → items Vec index
    visible_indices: Vec<usize>,

    // Cache loaded textures so we don't re-read from disk on every rebuild
    icon_cache: HashMap<String, gdk::Texture>,
    is_rebuilding: bool,
    // Widget references
    list_box: gtk::ListBox,
    sidebar_scroll: gtk::ScrolledWindow,

    // Detail panel widget refs (imperative updates)
    detail_icon: gtk::Image,
    detail_name: gtk::Label,
    detail_version_row: adw::ActionRow,
    detail_filename_row: adw::ActionRow,
    detail_homepage_row: adw::ActionRow,
    detail_size_row: adw::ActionRow,
    detail_seed_row: adw::ActionRow,
    detail_last_played_row: adw::ActionRow,
    detail_description: gtk::Label,
    detail_remove_btn: gtk::Button,
    detail_box: gtk::Box,
    detail_placeholder: adw::StatusPage,

    download_status_bar: Controller<DownloadStatusBar>,
    toast_overlay: adw::ToastOverlay,
}

impl InstanceEditorDialog {
    fn focused_item(&self) -> Option<&EditorItem> {
        self.focused_index.and_then(|i| self.items.get(i))
    }

    fn checked_count(&self) -> usize {
        self.items.iter().filter(|i| i.is_checked).count()
    }

    fn type_icon(&self) -> &'static str {
        match &self.editor_type {
            EditorType::Mods => "application-x-addon-symbolic",
            EditorType::Components => "application-x-firmware-symbolic",
            EditorType::ResourcePacks => "package-x-generic-symbolic",
            EditorType::ShaderPacks => "package-x-generic-symbolic",
            EditorType::Worlds => "globe-symbolic",
        }
    }

    /// Should we show this item given the current search query?
    fn item_matches_search(&self, item: &EditorItem) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        let q = self.search_query.to_lowercase();
        item.name.to_lowercase().contains(&q)
            || item.filename.to_lowercase().contains(&q)
            || item.id.to_lowercase().contains(&q)
    }

    /// Update the detail panel to show the currently focused item.
    fn update_detail_panel(&self) {
        if let Some(item) = self.focused_item() {
            self.detail_placeholder.set_visible(false);
            self.detail_box.set_visible(true);

            // Icon — use cached texture if available
            if let Some(icon_path) = &item.icon_path {
                if let Some(tex) = self.icon_cache.get(icon_path.as_str()) {
                    self.detail_icon.set_paintable(Some(tex));
                    self.detail_icon.set_icon_name(None);
                } else {
                    self.detail_icon.set_paintable(gtk::gdk::Paintable::NONE);
                    self.detail_icon.set_icon_name(Some(self.type_icon()));
                }
            } else {
                self.detail_icon.set_paintable(gtk::gdk::Paintable::NONE);
                self.detail_icon.set_icon_name(Some(self.type_icon()));
            }

            self.detail_name.set_label(&item.name);

            // Version row
            if !item.version.is_empty() {
                self.detail_version_row.set_subtitle(&item.version);
                self.detail_version_row.set_visible(true);
            } else {
                self.detail_version_row.set_visible(false);
            }

            // Filename row
            if !item.filename.is_empty() {
                self.detail_filename_row.set_subtitle(&item.filename);
                self.detail_filename_row.set_visible(true);
            } else {
                self.detail_filename_row.set_visible(false);
            }

            // Homepage row
            if let Some(hp) = &item.homepage {
                self.detail_homepage_row.set_subtitle(hp);
                self.detail_homepage_row.set_visible(true);
            } else {
                self.detail_homepage_row.set_visible(false);
            }

            // Size row
            if let Some(size) = &item.size {
                self.detail_size_row.set_subtitle(size);
                self.detail_size_row.set_visible(true);
            } else {
                self.detail_size_row.set_visible(false);
            }

            // Seed row
            if let Some(seed) = &item.seed {
                self.detail_seed_row.set_subtitle(seed);
                self.detail_seed_row.set_visible(true);
            } else {
                self.detail_seed_row.set_visible(false);
            }

            // Last Played row
            if let Some(lp) = &item.last_played {
                self.detail_last_played_row.set_subtitle(lp);
                self.detail_last_played_row.set_visible(true);
            } else {
                self.detail_last_played_row.set_visible(false);
            }

            // Description
            if let Some(desc) = &item.description {
                self.detail_description.set_label(desc);
                self.detail_description.set_visible(true);
            } else {
                self.detail_description.set_visible(false);
            }

            // Remove button
            self.detail_remove_btn.set_visible(!self.multi_select);
        } else {
            self.detail_placeholder.set_visible(true);
            self.detail_box.set_visible(false);
        }
    }

    // -------------------------------------------------------------------
    // Full list rebuild — only called when the list content actually changes
    // (open, search, remove, multi-select toggle, select/deselect all)
    // -------------------------------------------------------------------
    fn rebuild_list(&mut self, sender: &relm4::ComponentSender<Self>) {
        self.is_rebuilding = true;
        // Pre-cache all icons before entering the loop (needs &mut self)
        for item in &self.items {
            if let Some(icon_path) = &item.icon_path {
                if !self.icon_cache.contains_key(icon_path.as_str()) {
                    if let Ok(tex) = gdk::Texture::from_filename(icon_path) {
                        self.icon_cache.insert(icon_path.clone(), tex);
                    }
                }
            }
        }

        let list_box = &self.list_box;

        // Save scroll position
        let vadj = self.sidebar_scroll.vadjustment();
        let scroll_pos = vadj.value();

        // Remove all children
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        self.visible_indices.clear();

        for (idx, item) in self.items.iter().enumerate() {
            if !self.item_matches_search(item) {
                continue;
            }

            let row = adw::ActionRow::builder()
                .title(&item.name)
                .activatable(true)
                .focusable(false)
                .build();

            // Subtitle
            if !item.version.is_empty() {
                row.set_subtitle(&item.version);
            } else if !item.filename.is_empty() {
                row.set_subtitle(&item.filename);
            }

            // Icon prefix (from cache — no &mut self needed)
            let icon_widget = if let Some(icon_path) = &item.icon_path {
                if let Some(tex) = self.icon_cache.get(icon_path.as_str()) {
                    let img = gtk::Image::from_paintable(Some(tex));
                    img.set_pixel_size(32);
                    img
                } else {
                    let img = gtk::Image::from_icon_name(self.type_icon());
                    img.set_pixel_size(24);
                    img
                }
            } else {
                let img = gtk::Image::from_icon_name(self.type_icon());
                img.set_pixel_size(24);
                img
            };
            row.add_prefix(&icon_widget);

            // Suffix
            if self.multi_select {
                let check = gtk::CheckButton::builder()
                    .active(item.is_checked)
                    .valign(gtk::Align::Center)
                    .can_focus(false)
                    .build();
                row.add_suffix(&check);
                row.set_activatable_widget(Some(&check));

                let sender_clone = sender.input_sender().clone();
                let actual_idx = idx;
                check.connect_toggled(move |btn| {
                    if sender_clone
                        .send(EditorInput::SetChecked(actual_idx, btn.is_active()))
                        .is_err()
                    {
                        // ignore error
                    }
                });
            }

            // We use row_selected on the ListBox instead for single selection

            list_box.append(&row);
            self.visible_indices.push(idx);
        }

        // Restore selection
        if let Some(focused) = self.focused_index {
            if let Some(pos) = self.visible_indices.iter().position(|&i| i == focused) {
                if let Some(row) = list_box.row_at_index(pos as i32) {
                    list_box.select_row(Some(&row));
                }
            }
        }

        // Restore scroll position
        gtk::glib::idle_add_local_once(move || {
            vadj.set_value(scroll_pos);
        });

        // Update detail panel
        self.update_detail_panel();
        self.is_rebuilding = false;
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[relm4::component(pub)]
impl SimpleComponent for InstanceEditorDialog {
    type Init = ();
    type Input = EditorInput;
    type Output = EditorOutput;

    view! {
        adw::Window {
            #[watch]
            set_title: Some(&model.title),
            set_default_width: 850,
            set_default_height: 550,
            set_modal: true,
            set_hide_on_close: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(EditorInput::Close);
                gtk::glib::Propagation::Stop
            },

            #[name = "toast_overlay"]
            #[wrap(Some)]
            set_content = &adw::ToastOverlay {
                adw::ToolbarView {

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::NavigationSplitView {
                        set_vexpand: true,
                        set_min_sidebar_width: 280.0,
                        set_max_sidebar_width: 400.0,

                        // Sidebar: item list
                        #[wrap(Some)]
                        set_sidebar = &adw::NavigationPage {
                            set_title: "Items",

                            #[wrap(Some)]
                            set_child = &adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {
                                    #[wrap(Some)]
                                    set_title_widget = &adw::WindowTitle {
                                        #[watch]
                                        set_title: &model.title,
                                        #[watch]
                                        set_subtitle: &{
                                            let count = model.items.len();
                                            let label = match model.editor_type {
                                                EditorType::Mods => "mod",
                                                EditorType::Components => "component",
                                                EditorType::ResourcePacks => "resource pack",
                                                EditorType::ShaderPacks => "shader pack",
                                                EditorType::Worlds => "world",
                                            };
                                            if count == 1 { format!("1 {}", label) } else { format!("{} {}s", count, label) }
                                        },
                                    },

                                    // Selection / Multi-select on the left
                                    pack_start = &gtk::ToggleButton {
                                        set_icon_name: "selection-mode-symbolic",
                                        set_tooltip_text: Some("Select Multiple"),
                                        #[watch]
                                        set_active: model.multi_select,
                                        connect_toggled[sender] => move |btn| {
                                            sender.input(EditorInput::SetMultiSelect(btn.is_active()));
                                        },
                                    },

                                    // Actions on the right
                                    pack_end = &gtk::Button {
                                        set_icon_name: "list-add-symbolic",
                                        set_tooltip_text: Some("Add..."),
                                        #[watch]
                                        set_visible: !matches!(model.editor_type, EditorType::Components),
                                        connect_clicked => EditorInput::AddItemsRequest,
                                    },
                                },

                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    // Search Bar
                                    gtk::SearchEntry {
                                        set_margin_all: 8,
                                        set_placeholder_text: Some("Search items..."),
                                        connect_search_changed[sender] => move |entry| {
                                            sender.input(EditorInput::Search(entry.text().to_string()));
                                        },
                                    },

                                    // Empty state
                                    adw::StatusPage {
                                        #[watch]
                                        set_visible: model.items.is_empty(),
                                        set_vexpand: true,
                                        #[watch]
                                        set_icon_name: Some(model.type_icon()),
                                        #[watch]
                                        set_title: match model.editor_type {
                                            EditorType::Mods => "No Mods",
                                            EditorType::Components => "No Components",
                                            EditorType::ResourcePacks => "No Resource Packs",
                                            EditorType::ShaderPacks => "No Shader Packs",
                                            EditorType::Worlds => "No Worlds",
                                        },
                                        #[watch]
                                        set_description: Some(match model.editor_type {
                                            EditorType::Components => "This instance has no extra components.",
                                            _ => "Drag &amp; drop files here or click + to add.",
                                        }),
                                    },

                                #[local_ref]
                                sidebar_scroll_ref -> gtk::ScrolledWindow {
                                    set_vexpand: true,
                                    #[watch]
                                    set_visible: !model.items.is_empty(),
                                    set_hscrollbar_policy: gtk::PolicyType::Never,

                                    #[local_ref]
                                    list_box_ref -> gtk::ListBox {
                                        #[watch]
                                        set_selection_mode: if model.multi_select { gtk::SelectionMode::None } else { gtk::SelectionMode::Single },
                                        set_css_classes: &["navigation-sidebar"],
                                        set_margin_start: 6,
                                        set_margin_end: 6,
                                        set_margin_bottom: 6,
                                    }
                                },
                            },
                            add_bottom_bar = model.download_status_bar.widget(),
                        },
                    },

                        // Content: detail panel
                        #[wrap(Some)]
                        set_content = &adw::NavigationPage {
                            set_title: "Details",

                            #[wrap(Some)]
                            set_child = &adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {
                                    set_show_end_title_buttons: true,

                                    pack_end = &gtk::Button {
                                        set_icon_name: "folder-open-symbolic",
                                        set_tooltip_text: Some("Open Folder"),
                                        #[watch]
                                        set_visible: !matches!(model.editor_type, EditorType::Components),
                                        connect_clicked => EditorInput::OpenFolder,
                                    },
                                },

                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    // No selection placeholder
                                    #[local_ref]
                                    detail_placeholder_ref -> adw::StatusPage {
                                        set_visible: true,
                                        set_vexpand: true,
                                        set_icon_name: Some("find-location-symbolic"),
                                        set_title: "Select an Item",
                                        set_description: Some("Click an item to view its details."),
                                    },

                                    // Detail view (imperatively updated)
                                    #[local_ref]
                                    detail_box_ref -> gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_visible: false,
                                        set_margin_all: 24,
                                        set_spacing: 16,
                                        set_vexpand: true,

                                        // Icon + title centered
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 8,
                                            set_halign: gtk::Align::Center,
                                            set_margin_top: 16,

                                            #[local_ref]
                                            detail_icon_ref -> gtk::Image {
                                                set_pixel_size: 64,
                                                set_css_classes: &["dim-label"],
                                            },

                                            #[local_ref]
                                            detail_name_ref -> gtk::Label {
                                                set_css_classes: &["title-3"],
                                                set_wrap: true,
                                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                                set_justify: gtk::Justification::Center,
                                                set_max_width_chars: 30,
                                            },
                                        },

                                        // Info rows
                                        adw::PreferencesGroup {
                                            #[local_ref]
                                            detail_version_row_ref -> adw::ActionRow {
                                                set_title: "Version",
                                            },

                                            #[local_ref]
                                            detail_filename_row_ref -> adw::ActionRow {
                                                set_title: "Filename",
                                            },

                                            #[local_ref]
                                            detail_homepage_row_ref -> adw::ActionRow {
                                                set_title: "Homepage",
                                            },

                                            #[local_ref]
                                            detail_size_row_ref -> adw::ActionRow {
                                                set_title: "Size",
                                            },

                                            #[local_ref]
                                            detail_seed_row_ref -> adw::ActionRow {
                                                set_title: "Seed",
                                            },

                                            #[local_ref]
                                            detail_last_played_row_ref -> adw::ActionRow {
                                                set_title: "Last Played",
                                            },
                                        },

                                        // Description
                                        #[local_ref]
                                        detail_description_ref -> gtk::Label {
                                            set_visible: false,
                                            set_wrap: true,
                                            set_halign: gtk::Align::Start,
                                            set_css_classes: &["dim-label", "body"],
                                            set_margin_top: 4,
                                        },

                                        // Spacer
                                        gtk::Box { set_vexpand: true },

                                        // Remove focused item button
                                        #[local_ref]
                                        detail_remove_btn_ref -> gtk::Button {
                                            set_label: "Remove",
                                            set_css_classes: &["destructive-action", "pill"],
                                            set_halign: gtk::Align::Center,
                                            set_margin_bottom: 8,
                                            set_visible: false,
                                            connect_clicked => EditorInput::RemoveFocused,
                                        },
                                    },
                                },
                            },
                        },
                    },
                },

                add_bottom_bar = &gtk::Revealer {
                    #[watch]
                    set_reveal_child: model.multi_select,
                    set_transition_type: gtk::RevealerTransitionType::SlideUp,

                    gtk::ActionBar {
                        set_halign: gtk::Align::Fill,
                        pack_start = &gtk::Button {
                            set_label: "Select All",
                            set_tooltip_text: Some("Select All (Ctrl+A)"),
                            set_css_classes: &["flat"],
                            connect_clicked => EditorInput::SelectAll,
                        },
                        pack_start = &gtk::Button {
                            set_label: "Select None",
                            set_tooltip_text: Some("Deselect All"),
                            set_css_classes: &["flat"],
                            connect_clicked => EditorInput::DeselectAll,
                        },

                        #[wrap(Some)]
                        set_center_widget = &gtk::Label {
                            #[watch]
                            set_label: &format!("{} selected", model.checked_count()),
                            set_css_classes: &["dim-label"],
                        },

                        pack_end = &gtk::Button {
                            #[watch]
                            set_label: &format!("Remove Selected ({})", model.checked_count()),
                            set_css_classes: &["destructive-action"],
                            #[watch]
                            set_sensitive: model.items.iter().any(|i| i.is_checked),
                            connect_clicked => EditorInput::RemoveSelected,
                        },
                    },
                },
            }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_box = gtk::ListBox::new();
        let sidebar_scroll = gtk::ScrolledWindow::new();

        // Create detail panel widgets
        let detail_icon = gtk::Image::new();
        let detail_name = gtk::Label::new(None);
        let detail_version_row = adw::ActionRow::new();
        let detail_filename_row = adw::ActionRow::new();
        let detail_homepage_row = adw::ActionRow::new();
        let detail_size_row = adw::ActionRow::new();
        let detail_seed_row = adw::ActionRow::new();
        let detail_last_played_row = adw::ActionRow::new();
        let detail_description = gtk::Label::new(None);
        let detail_remove_btn = gtk::Button::new();
        let detail_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        let detail_placeholder = adw::StatusPage::new();

        let download_status_bar = DownloadStatusBar::builder().launch(()).detach();

        let mut model = InstanceEditorDialog {
            visible: false,
            title: String::new(),
            editor_type: EditorType::Mods,
            items: Vec::new(),
            focused_index: None,
            multi_select: false,
            search_query: String::new(),
            visible_indices: Vec::new(),
            is_rebuilding: false,
            icon_cache: HashMap::new(),
            list_box: list_box.clone(),
            sidebar_scroll: sidebar_scroll.clone(),
            detail_icon: detail_icon.clone(),
            detail_name: detail_name.clone(),
            detail_version_row: detail_version_row.clone(),
            detail_filename_row: detail_filename_row.clone(),
            detail_homepage_row: detail_homepage_row.clone(),
            detail_size_row: detail_size_row.clone(),
            detail_seed_row: detail_seed_row.clone(),
            detail_last_played_row: detail_last_played_row.clone(),
            detail_description: detail_description.clone(),
            detail_remove_btn: detail_remove_btn.clone(),
            detail_box: detail_box.clone(),
            detail_placeholder: detail_placeholder.clone(),
            download_status_bar,
            toast_overlay: adw::ToastOverlay::new(),
        };

        let list_box_ref = &model.list_box;
        let sidebar_scroll_ref = &model.sidebar_scroll;
        let detail_icon_ref = &model.detail_icon;
        let detail_name_ref = &model.detail_name;
        let detail_version_row_ref = &model.detail_version_row;
        let detail_filename_row_ref = &model.detail_filename_row;
        let detail_homepage_row_ref = &model.detail_homepage_row;
        let detail_size_row_ref = &model.detail_size_row;
        let detail_seed_row_ref = &model.detail_seed_row;
        let detail_last_played_row_ref = &model.detail_last_played_row;
        let detail_description_ref = &model.detail_description;
        let detail_remove_btn_ref = &model.detail_remove_btn;
        let detail_box_ref = &model.detail_box;
        let detail_placeholder_ref = &model.detail_placeholder;
        let widgets = view_output!();

        // --- Keyboard shortcut: Ctrl+A, Delete, Escape ---
        let key_controller = gtk::EventControllerKey::new();
        {
            let sender_clone = sender.input_sender().clone();
            key_controller.connect_key_pressed(move |_, keyval, _, state| {
                sender_clone
                    .send(EditorInput::KeyPressed(keyval, state))
                    .ok();
                gtk::glib::Propagation::Proceed
            });
        }
        root.add_controller(key_controller);

        // --- Drag & Drop ---
        let drop_target =
            gtk::DropTarget::new(gtk::gio::File::static_type(), gdk::DragAction::COPY);
        {
            let sender_clone = sender.input_sender().clone();
            drop_target.connect_drop(move |_, value, _x, _y| {
                if let Ok(file) = value.get::<gtk::gio::File>() {
                    if let Some(path) = file.path() {
                        sender_clone
                            .send(EditorInput::FilesDropped(vec![path]))
                            .ok();
                        return true;
                    }
                }
                false
            });
        }
        // --- Selection change ---
        {
            let sender_clone = sender.clone();
            list_box.connect_row_selected(move |_lb, row| {
                if let Some(row) = row {
                    let idx = row.index();
                    sender_clone.input(EditorInput::FocusItem(idx as usize));
                }
            });
        }

        root.add_controller(drop_target);

        model.toast_overlay = widgets.toast_overlay.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            EditorInput::Open(editor_type, title, items) => {
                println!("EditorInput::Open: starting...");
                self.visible = true;
                self.editor_type = editor_type;
                self.title = title;
                self.items = items;
                self.focused_index = None;
                self.multi_select = false;
                self.search_query = String::new();
                self.icon_cache.clear(); // New item set → fresh cache
                println!("EditorInput::Open: calling rebuild_list");
                self.rebuild_list(&sender);
                println!("EditorInput::Open: done");
            }
            EditorInput::Close => {
                self.visible = false;
            }
            EditorInput::Search(query) => {
                self.search_query = query;
                self.rebuild_list(&sender);
            }
            
            EditorInput::UpdateItems(items) => {
                self.items = items;
                self.rebuild_list(&sender);
            }
            EditorInput::DownloadProgress(status, progress, visible) => {
                self.download_status_bar.emit(DownloadStatusBarInput::Update(status, progress, visible));
            }
            EditorInput::ShowToast(msg) => {
                self.toast_overlay.add_toast(adw::Toast::new(&msg));
            }

            // ---------------------------------------------------------------
            // Focus / check: rebuild list (fast thanks to icon cache)
            // ---------------------------------------------------------------
            EditorInput::FocusItem(index) => {
                if self.is_rebuilding {
                    return;
                }
                if index < self.visible_indices.len() {
                    let actual_idx = self.visible_indices[index];
                    if self.multi_select {
                        if let Some(item) = self.items.get_mut(actual_idx) {
                            item.is_checked = !item.is_checked;
                        }
                        self.rebuild_list(&sender);
                    } else {
                        self.focused_index = Some(actual_idx);
                        self.update_detail_panel();
                    }
                }
            }
            EditorInput::ToggleCheck(index) => {
                if let Some(item) = self.items.get_mut(index) {
                    item.is_checked = !item.is_checked;
                }
                self.rebuild_list(&sender);
            }
            EditorInput::SetChecked(index, checked) => {
                if let Some(item) = self.items.get_mut(index) {
                    item.is_checked = checked;
                }
            }

            // ---------------------------------------------------------------
            // These change many rows → full rebuild
            // ---------------------------------------------------------------
            EditorInput::SelectAll => {
                for item in &mut self.items {
                    item.is_checked = true;
                }
                self.rebuild_list(&sender);
            }
            EditorInput::DeselectAll => {
                for item in &mut self.items {
                    item.is_checked = false;
                }
                self.rebuild_list(&sender);
            }
            EditorInput::SetMultiSelect(active) => {
                if self.multi_select != active {
                    self.multi_select = active;
                    if !self.multi_select {
                        for item in &mut self.items {
                            item.is_checked = false;
                        }
                    }
                    self.rebuild_list(&sender);
                }
            }
            EditorInput::RemoveFocused => {
                if let Some(idx) = self.focused_index {
                    if let Some(item) = self.items.get(idx) {
                        let id = item.id.clone();
                        let name = item.name.clone();
                        let window = self.list_box.root().and_downcast::<gtk::Window>();

                        let dialog = adw::AlertDialog::builder()
                            .heading("Confirm Removal")
                            .body(&format!("Are you sure you want to remove '{}'?", name))
                            .build();

                        dialog.add_response("cancel", "Cancel");
                        dialog.add_response("remove", "Remove");
                        dialog.set_response_appearance(
                            "remove",
                            adw::ResponseAppearance::Destructive,
                        );
                        dialog.set_default_response(Some("cancel"));
                        dialog.set_close_response("cancel");

                        let sender_clone = sender.input_sender().clone();
                        dialog.connect_response(None, move |_d, response| {
                            if response == "remove" {
                                sender_clone
                                    .send(EditorInput::ConfirmRemove(vec![id.clone()]))
                                    .ok();
                            }
                        });
                        if let Some(w) = window {
                            dialog.present(Some(&w));
                        } else {
                            dialog.present(None::<&gtk::Widget>);
                        }
                    }
                }
            }
            EditorInput::OpenFolder => {
                sender
                    .output(EditorOutput::OpenFolder(self.editor_type.clone()))
                    .ok();
            }
            EditorInput::BrowseModrinth => {
                sender
                    .output(EditorOutput::BrowseModrinth(self.editor_type.clone()))
                    .ok();
            }
            EditorInput::RemoveSelected => {
                let ids: Vec<String> = self
                    .items
                    .iter()
                    .filter(|item| item.is_checked)
                    .map(|item| item.id.clone())
                    .collect();

                if ids.is_empty() {
                    return;
                }

                let count = ids.len();
                let window = self.list_box.root().and_downcast::<gtk::Window>();

                let dialog = adw::AlertDialog::builder()
                    .heading("Confirm Removal")
                    .body(&format!(
                        "Are you sure you want to remove {} selected items?",
                        count
                    ))
                    .build();

                dialog.add_response("cancel", "Cancel");
                dialog.add_response("remove", "Remove");
                dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");

                let sender_clone = sender.input_sender().clone();
                let ids_clone = ids.clone();
                dialog.connect_response(None, move |_d, response| {
                    if response == "remove" {
                        sender_clone
                            .send(EditorInput::ConfirmRemove(ids_clone.clone()))
                            .ok();
                    }
                });
                if let Some(w) = window {
                    dialog.present(Some(&w));
                } else {
                    dialog.present(None::<&gtk::Widget>);
                }
            }
            EditorInput::ConfirmRemove(ids) => {
                println!("ConfirmRemove: Removing {} items", ids.len());
                let output = match &self.editor_type {
                    EditorType::Mods => EditorOutput::RemoveMods(ids.clone()),
                    EditorType::Components => EditorOutput::RemoveComponents(ids.clone()),
                    EditorType::ResourcePacks => EditorOutput::RemoveResourcePacks(ids.clone()),
                    EditorType::ShaderPacks => EditorOutput::RemoveShaderPacks(ids.clone()),
                    EditorType::Worlds => EditorOutput::RemoveWorlds(ids.clone()),
                };
                let sender_clone = sender.clone();
                gtk::glib::idle_add_local_once(move || {
                    println!("ConfirmRemove: idle_add_local_once executing!");
                    if let Err(e) = sender_clone.output(output) {
                        eprintln!("Failed to send EditorOutput: {:?}", e);
                    }
                    println!("ConfirmRemove: idle_add_local_once finished!");
                });

                // Clear focused state before modifying the list
                self.focused_index = None;
                self.items.retain(|item| !ids.contains(&item.id));

                if self.items.is_empty() {
                    self.multi_select = false;
                }

                println!("ConfirmRemove: Rebuilding list...");
                self.rebuild_list(&sender);
                println!("ConfirmRemove: Done.");
            }
            EditorInput::AddItemsRequest => {
                let (title, label, suffixes) = match self.editor_type {
                    EditorType::Mods => ("Select Mods to Add", "Add Mods", vec!["jar"]),
                    EditorType::ResourcePacks => (
                        "Select Resource Packs to Add",
                        "Add Resource Packs",
                        vec!["zip"],
                    ),
                    EditorType::ShaderPacks => (
                        "Select Shader Packs to Add",
                        "Add Shader Packs",
                        vec!["zip"],
                    ),
                    EditorType::Worlds => ("Select Worlds to Add", "Add Worlds", vec![]),
                    _ => ("Select Items to Add", "Add", vec![]),
                };

                let dialog = gtk::FileDialog::builder()
                    .title(title)
                    .accept_label(label)
                    .modal(true)
                    .build();

                if !suffixes.is_empty() {
                    let filter = gtk::FileFilter::new();
                    for s in suffixes {
                        filter.add_suffix(s);
                    }
                    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                    filters.append(&filter);
                    dialog.set_filters(Some(&filters));
                }

                let sender_clone = sender.input_sender().clone();
                let window = self.list_box.root().and_downcast::<gtk::Window>();

                if matches!(self.editor_type, EditorType::Worlds) {
                    dialog.select_folder(
                        window.as_ref(),
                        None::<&gtk::gio::Cancellable>,
                        move |res| {
                            if let Ok(folder) = res {
                                if let Some(path) = folder.path() {
                                    let _ = sender_clone.send(EditorInput::AddItems(vec![path]));
                                }
                            }
                        },
                    );
                } else {
                    dialog.open_multiple(
                        window.as_ref(),
                        None::<&gtk::gio::Cancellable>,
                        move |res| {
                            if let Ok(files) = res {
                                let mut paths = Vec::new();
                                for i in 0..files.n_items() {
                                    if let Some(item) = files.item(i) {
                                        if let Ok(file) = item.downcast::<gtk::gio::File>() {
                                            if let Some(path) = file.path() {
                                                paths.push(path);
                                            }
                                        }
                                    }
                                }
                                if !paths.is_empty() {
                                    let _ = sender_clone.send(EditorInput::AddItems(paths));
                                }
                            }
                        },
                    );
                }
            }
            EditorInput::AddItems(paths) => {
                sender
                    .output(EditorOutput::AddItems(self.editor_type.clone(), paths))
                    .ok();
                self.visible = false;
            }
            EditorInput::KeyPressed(key, modifiers) => {
                let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
                match key {
                    gdk::Key::a if ctrl => {
                        if !self.multi_select {
                            self.multi_select = true;
                        }
                        for item in &mut self.items {
                            item.is_checked = true;
                        }
                        self.rebuild_list(&sender);
                    }
                    gdk::Key::Delete | gdk::Key::BackSpace => {
                        if self.multi_select && self.checked_count() > 0 {
                            sender.input(EditorInput::RemoveSelected);
                        } else if self.focused_index.is_some() {
                            sender.input(EditorInput::RemoveFocused);
                        }
                    }
                    gdk::Key::Escape => {
                        if self.multi_select {
                            self.multi_select = false;
                            for item in &mut self.items {
                                item.is_checked = false;
                            }
                            self.rebuild_list(&sender);
                        } else {
                            self.visible = false;
                        }
                    }
                    _ => {}
                }
            }
            EditorInput::FilesDropped(paths) => {
                if !matches!(self.editor_type, EditorType::Components) {
                    sender
                        .output(EditorOutput::AddItems(self.editor_type.clone(), paths))
                        .ok();
                    self.visible = false;
                }
            }
        }
    }
}
