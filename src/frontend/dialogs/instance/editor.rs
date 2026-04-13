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
    pub enabled: bool,
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
    FocusVisibleItem(usize, bool, bool, bool), // index, is_shift, is_ctrl, is_click
    ToggleCheck(usize),
    SetChecked(usize, bool),
    SelectAll,
    DeselectAll,

    // Mode
    SetMultiSelect(bool),

    // Actions
    RemoveSelected,
    RemoveRequest(Option<usize>),
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
    ToggleFocusedModEnabled,
    SetSelectedModsEnabled(bool),
    RenameWorldRequest(usize),
    RenameWorld(usize, String),
    MoveItemRequest(usize),
    CopyItemRequest(usize),
    MoveSelectedRequest,
    CopySelectedRequest,
    ShowContextMenu(usize, f64, f64),
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
    SetModsEnabled(Vec<String>, bool),
    RenameWorld(String, String), // folder, new_name
    MoveItems(EditorType, Vec<String>), 
    CopyItems(EditorType, Vec<String>),
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
    selection_anchor: Option<usize>,
    selection_initial_state: Option<std::collections::HashSet<String>>, // IDs of checked items before range
    selection_range_mode: bool, // true = checking, false = unchecking

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
    detail_toggle_enabled_btn: gtk::Button,
    detail_box: gtk::Box,
    detail_placeholder: adw::StatusPage,

    context_menu: gtk::Popover,
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

            // Version row (titled "Format" for RPs)
            if matches!(self.editor_type, EditorType::ResourcePacks) {
                self.detail_version_row.set_title("Format");
            } else {
                self.detail_version_row.set_title("Version");
            }

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

            // Buttons visibility
            let is_world = matches!(self.editor_type, EditorType::Worlds);
            let is_pack = matches!(self.editor_type, EditorType::ResourcePacks | EditorType::ShaderPacks);
            self.detail_remove_btn.set_visible(!self.multi_select && !is_world && !is_pack);

            if matches!(self.editor_type, EditorType::Mods) {
                self.detail_toggle_enabled_btn.set_visible(!self.multi_select);
                if item.enabled {
                    self.detail_toggle_enabled_btn.set_label("Disable");
                    self.detail_toggle_enabled_btn.set_icon_name("list-remove-symbolic");
                } else {
                    self.detail_toggle_enabled_btn.set_label("Enable");
                    self.detail_toggle_enabled_btn.set_icon_name("list-add-symbolic");
                }
            } else {
                self.detail_toggle_enabled_btn.set_visible(false);
            }
        } else {
            self.detail_placeholder.set_visible(true);
            self.detail_box.set_visible(false);
        }
    }

    fn create_context_menu_box(&self, index: usize, sender_clone: relm4::Sender<EditorInput>) -> gtk::Box {
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 4);
        box_.add_css_class("menu-box");
        box_.set_width_request(160);

        let is_world = matches!(self.editor_type, EditorType::Worlds);

        if is_world {
            let btn_rename = build_menu_item("Rename...", false);
            let s_clone = sender_clone.clone();
            btn_rename.connect_clicked(move |_| {
                s_clone.send(EditorInput::RenameWorldRequest(index)).ok();
            });
            box_.append(&btn_rename);
        }

        let btn_move = build_menu_item("Move to...", false);
        let s_clone = sender_clone.clone();
        btn_move.connect_clicked(move |_| {
            s_clone.send(EditorInput::MoveItemRequest(index)).ok();
        });

        let btn_copy = build_menu_item("Copy to...", false);
        let s_clone = sender_clone.clone();
        btn_copy.connect_clicked(move |_| {
            s_clone.send(EditorInput::CopyItemRequest(index)).ok();
        });

        let btn_remove = build_menu_item("Remove", true);
        let s_clone = sender_clone.clone();
        btn_remove.connect_clicked(move |_| {
            s_clone.send(EditorInput::RemoveRequest(Some(index))).ok();
        });

        box_.append(&btn_move);
        box_.append(&btn_copy);
        box_.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        box_.append(&btn_remove);

        box_
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

        // Unparent persistent context menu to avoid warnings/crashes during destruction
        if self.context_menu.parent().is_some() {
            self.context_menu.popdown();
            self.context_menu.unparent();
        }

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

            // Subtitle - only show if it's different from the title or provides new info
            if !item.version.is_empty() {
                row.set_subtitle(&item.version);
            } else if !item.filename.is_empty() && item.filename != item.name {
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

            if !item.enabled {
                row.add_css_class("dim-label");
            }

            if self.multi_select && self.focused_index == Some(idx) {
                row.add_css_class("selected");
            }

            let actual_idx = idx;
            let s_clone = sender.input_sender().clone();
            
            // Immediate selection gesture for mouse clicks
            let select_gesture = gtk::GestureClick::builder()
                .button(1) // Left click only
                .build();
            select_gesture.connect_pressed(move |gesture, _, _, _| {
                let (shift, ctrl) = gesture.current_event()
                    .map(|e| {
                        let mods = e.modifier_state();
                        (mods.contains(gdk::ModifierType::SHIFT_MASK), mods.contains(gdk::ModifierType::CONTROL_MASK))
                    })
                    .unwrap_or((false, false));
                s_clone.send(EditorInput::FocusVisibleItem(actual_idx, shift, ctrl, true)).ok();
            });
            row.add_controller(select_gesture);

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
                check.connect_toggled(move |btn| {
                    if sender_clone
                        .send(EditorInput::SetChecked(actual_idx, btn.is_active()))
                        .is_err()
                    {
                        // ignore error
                    }
                });
            }



            // Context menus and action buttons for worlds and packs
            let has_menu = matches!(self.editor_type, EditorType::Worlds | EditorType::ResourcePacks | EditorType::ShaderPacks);
            
            if has_menu && !self.multi_select {
                // Action button suffix
                let menu_button = gtk::MenuButton::builder()
                    .icon_name("view-more-symbolic")
                    .valign(gtk::Align::Center)
                    .css_classes(vec!["flat", "circular"])
                    .tooltip_text("More Actions")
                    .build();
                
                let s_clone = sender.input_sender().clone();
                
                // Build the menu (reused for button popover)
                let menu_box = self.create_context_menu_box(actual_idx, s_clone);
                
                let popover = gtk::Popover::new();
                popover.set_child(Some(&menu_box));
                menu_button.set_popover(Some(&popover));
                row.add_suffix(&menu_button);

                // Right-click support
                let gesture = gtk::GestureClick::builder()
                    .button(3) // Right click
                    .build();
                let sender_clone = sender.input_sender().clone();
                gesture.connect_pressed(move |_, _, x, y| {
                    sender_clone.send(EditorInput::ShowContextMenu(actual_idx, x, y)).ok();
                });
                row.add_controller(gesture);
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
                                        set_selection_mode: gtk::SelectionMode::Single,
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

                                        gtk::Box { set_vexpand: true },

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_spacing: 8,
                                            set_halign: gtk::Align::Center,
                                            set_margin_bottom: 12,

                                            #[local_ref]
                                            detail_toggle_enabled_btn_ref -> gtk::Button {
                                                set_label: "Disable",
                                                set_icon_name: "list-remove-symbolic",
                                                set_css_classes: &["pill"],
                                                set_visible: false,
                                                connect_clicked => EditorInput::ToggleFocusedModEnabled,
                                            },

                                            #[local_ref]
                                            detail_remove_btn_ref -> gtk::Button {
                                                set_label: "Remove",
                                                set_css_classes: &["destructive-action", "pill"],
                                                set_visible: false,
                                                connect_clicked => EditorInput::RemoveRequest(None),
                                            },
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

                        pack_end = &gtk::MenuButton {
                            set_icon_name: "document-send-symbolic",
                            set_tooltip_text: Some("Move/Copy Selected"),
                            set_direction: gtk::ArrowType::Up,
                            #[watch]
                            set_visible: !matches!(model.editor_type, EditorType::Components),
                            #[watch]
                            set_sensitive: model.items.iter().any(|i| i.is_checked),
                            #[wrap(Some)]
                            set_popover = &gtk::Popover {
                                set_position: gtk::PositionType::Top,
                                set_autohide: true,
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_margin_all: 0,
                                    set_spacing: 4,

                                    gtk::Button {
                                        set_label: "Move Selected to...",
                                        set_css_classes: &["flat", "menu-btn"],
                                        connect_clicked[sender] => move |btn| {
                                            btn.parent().and_then(|p| p.parent()).and_then(|p| p.downcast::<gtk::Popover>().ok()).map(|p| p.popdown());
                                            sender.input(EditorInput::MoveSelectedRequest);
                                        }
                                    },
                                    gtk::Button {
                                        set_label: "Copy Selected to...",
                                        set_css_classes: &["flat", "menu-btn"],
                                        connect_clicked[sender] => move |btn| {
                                            btn.parent().and_then(|p| p.parent()).and_then(|p| p.downcast::<gtk::Popover>().ok()).map(|p| p.popdown());
                                            sender.input(EditorInput::CopySelectedRequest);
                                        }
                                    },
                                }
                            }
                        },

                        pack_end = &gtk::Button {
                            set_label: "Enable",
                            set_css_classes: &["suggested-action"],
                            #[watch]
                            set_visible: matches!(model.editor_type, EditorType::Mods) && model.items.iter().any(|i| i.is_checked && !i.enabled),
                            connect_clicked => EditorInput::SetSelectedModsEnabled(true),
                        },

                        pack_end = &gtk::Button {
                            set_label: "Disable",
                            #[watch]
                            set_visible: matches!(model.editor_type, EditorType::Mods) && model.items.iter().any(|i| i.is_checked && i.enabled),
                            connect_clicked => EditorInput::SetSelectedModsEnabled(false),
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
        let detail_toggle_enabled_btn = gtk::Button::new();
        let detail_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
        let detail_placeholder = adw::StatusPage::new();
        let context_menu = gtk::Popover::new();

        let download_status_bar = DownloadStatusBar::builder().launch(()).detach();

        let mut model = InstanceEditorDialog {
            visible: false,
            title: String::new(),
            editor_type: EditorType::Mods,
            items: Vec::new(),
            focused_index: None,
            multi_select: false,
            search_query: String::new(),
            selection_anchor: None,
            selection_initial_state: None,
            selection_range_mode: true,
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
            detail_toggle_enabled_btn: detail_toggle_enabled_btn.clone(),
            detail_box: detail_box.clone(),
            detail_placeholder: detail_placeholder.clone(),
            context_menu: context_menu.clone(),
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
        let detail_toggle_enabled_btn_ref = &model.detail_toggle_enabled_btn;
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
        // --- Selection change (Keyboard only) ---
        {
            let sender_clone = sender.input_sender().clone();
            
            list_box.connect_row_selected(move |_lb, row| {
                if let Some(row) = row {
                    let idx = row.index();
                    
                    // Only handle if NOT a mouse event (clicks handled by per-row gestures)
                    let display = gdk::Display::default().unwrap();
                    let is_mouse = display.default_seat()
                        .and_then(|s| s.pointer())
                        .map(|d| d.modifier_state().intersects(gdk::ModifierType::BUTTON1_MASK | gdk::ModifierType::BUTTON2_MASK | gdk::ModifierType::BUTTON3_MASK))
                        .unwrap_or(false);
                    
                    if !is_mouse {
                        sender_clone.send(EditorInput::FocusVisibleItem(idx as usize, false, false, false)).ok();
                    }
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
                self.visible = true;
                self.editor_type = editor_type;
                self.title = title;
                self.items = items;
                self.focused_index = None;
                self.multi_select = false;
                self.search_query = String::new();
                self.icon_cache.clear(); // New item set → fresh cache
                self.rebuild_list(&sender);
            }
            EditorInput::ToggleFocusedModEnabled => {
                if let Some(focused) = self.focused_index {
                    if let Some(item) = self.items.get_mut(focused) {
                        item.enabled = !item.enabled;
                        sender
                            .output_sender()
                            .send(EditorOutput::SetModsEnabled(vec![item.filename.clone()], item.enabled))
                            .ok();
                    }
                }
            }
            EditorInput::SetSelectedModsEnabled(enable) => {
                let filenames: Vec<String> = self.items.iter()
                    .filter(|i| i.is_checked)
                    .map(|i| i.filename.clone())
                    .collect();
                
                if !filenames.is_empty() {
                    sender.output_sender().send(EditorOutput::SetModsEnabled(filenames, enable)).ok();
                }
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
            EditorInput::FocusVisibleItem(index, is_shift, is_ctrl, is_click) => {
                if let Some(&actual_idx) = self.visible_indices.get(index) {
                    // Update focus if changed
                    let focus_changed = self.focused_index != Some(actual_idx);
                    
                    if is_shift {
                        // Range selection
                        let entering_multiselect = !self.multi_select;
                        if entering_multiselect {
                            self.multi_select = true;
                        }
                        
                        // Determine anchor: either explicit anchor or the currently focused visible index
                        let anchor_opt = self.selection_anchor.or_else(|| {
                            self.focused_index.and_then(|fi| {
                                self.visible_indices.iter().position(|&idx| idx == fi)
                            })
                        });

                        if let Some(anchor_idx) = anchor_opt {
                            // If this is the FIRST shift click of a "session", capture the current state
                            if self.selection_initial_state.is_none() {
                                let mut initial_state = std::collections::HashSet::new();
                                for item in &self.items {
                                    if item.is_checked {
                                        initial_state.insert(item.id.clone());
                                    }
                                }
                                self.selection_initial_state = Some(initial_state);
                                
                                // Also ensure the anchor itself is recorded as the starting point if not already
                                self.selection_anchor = Some(anchor_idx);
                                
                                // Determine mode based on anchor, but force 'check' if we just entered multiselect
                                if entering_multiselect {
                                    self.selection_range_mode = true;
                                } else if let Some(&raw_idx) = self.visible_indices.get(anchor_idx) {
                                    if let Some(item) = self.items.get(raw_idx) {
                                        self.selection_range_mode = item.is_checked;
                                    }
                                }
                            }
                            
                            // Restore initial state before applying the current range
                            if let Some(initial_state) = &self.selection_initial_state {
                                for item in &mut self.items {
                                    item.is_checked = initial_state.contains(&item.id);
                                }
                            }
                            
                            // Apply the new range using the determined mode
                            let start = anchor_idx.min(index);
                            let end = anchor_idx.max(index);
                            let mode = self.selection_range_mode;
                            
                            for i in start..=end {
                                if let Some(&raw_idx) = self.visible_indices.get(i) {
                                    if let Some(item) = self.items.get_mut(raw_idx) {
                                        item.is_checked = mode;
                                    }
                                }
                            }
                            
                            // Focus usually moves to the clicked item in range select too
                            if focus_changed {
                                self.focused_index = Some(actual_idx);
                                self.update_detail_panel();
                            }
                            
                            self.rebuild_list(&sender);
                        } else {
                            // No anchor or focus yet, just toggle current and set as anchor
                            if let Some(item) = self.items.get_mut(actual_idx) {
                                item.is_checked = !item.is_checked;
                            }
                            self.selection_anchor = Some(index);
                            
                            if focus_changed {
                                self.focused_index = Some(actual_idx);
                                self.update_detail_panel();
                            }
                            
                            self.rebuild_list(&sender);
                        }
                    } else {
                        // Normal selection (moves anchor and clears initial state tracking)
                        self.selection_anchor = Some(index);
                        self.selection_initial_state = None;
                        
                        if self.multi_select {
                            let mut changed = false;
                            
                            // Toggle checkbox ONLY on actual clicks
                            if is_click {
                                if let Some(item) = self.items.get_mut(actual_idx) {
                                    item.is_checked = !item.is_checked;
                                    changed = true;
                                }
                            }
                            
                            // User wants normal click in multiselect to ALSO open details panel
                            // CTRL override avoids this.
                            if !is_ctrl && focus_changed {
                                self.focused_index = Some(actual_idx);
                                self.update_detail_panel();
                                changed = true;
                            }
                            
                            if changed {
                                self.rebuild_list(&sender);
                            }
                        } else {
                            if focus_changed {
                                self.focused_index = Some(actual_idx);
                                self.update_detail_panel();
                                self.rebuild_list(&sender); 
                                
                                // Sync ListBox selection
                                if let Some(row) = self.list_box.row_at_index(index as i32) {
                                    if self.list_box.selected_row().as_ref() != Some(&row) {
                                        self.list_box.select_row(Some(&row));
                                    }
                                }
                            }
                        }
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
            EditorInput::RemoveRequest(index_opt) => {
                let target_idx = index_opt.or(self.focused_index);
                if let Some(idx) = target_idx {
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
                            sender.input(EditorInput::RemoveRequest(None));
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
            EditorInput::RenameWorldRequest(index) => {
                if let Some(item) = self.items.get(index) {
                    let current_name = item.name.clone();
                    let window = self.list_box.root().and_downcast::<gtk::Window>();
                    let dialog = adw::AlertDialog::builder()
                        .heading("Rename World")
                        .body("Enter a new display name for the world:")
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("rename", "Rename");
                    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
                    
                    let entry = gtk::Entry::builder()
                        .text(&current_name)
                        .activates_default(true)
                        .build();
                    dialog.set_extra_child(Some(&entry));
                    
                    let sender_clone = sender.input_sender().clone();
                    dialog.connect_response(None, move |_, response| {
                        if response == "rename" {
                            let new_name = entry.text().to_string();
                            sender_clone.send(EditorInput::RenameWorld(index, new_name)).ok();
                        }
                    });
                    dialog.present(window.as_ref());
                }
            }
            EditorInput::RenameWorld(index, new_name) => {
                if let Some(item) = self.items.get_mut(index) {
                    let folder = item.id.clone();
                    item.name = new_name.clone();
                    sender.output_sender().send(EditorOutput::RenameWorld(folder, new_name)).ok();
                    self.rebuild_list(&sender);
                }
            }
            EditorInput::MoveItemRequest(index) => {
                 if let Some(item) = self.items.get(index) {
                    sender.output(EditorOutput::MoveItems(self.editor_type.clone(), vec![item.id.clone()])).ok();
                    self.visible = false;
                }
            }
            EditorInput::CopyItemRequest(index) => {
                if let Some(item) = self.items.get(index) {
                    sender.output(EditorOutput::CopyItems(self.editor_type.clone(), vec![item.id.clone()])).ok();
                    self.visible = false;
                }
            }
            EditorInput::MoveSelectedRequest => {
                let ids: Vec<String> = self.items.iter()
                    .filter(|i| i.is_checked)
                    .map(|i| i.id.clone())
                    .collect();
                if !ids.is_empty() {
                    sender.output(EditorOutput::MoveItems(self.editor_type.clone(), ids)).ok();
                    self.visible = false;
                }
            }
            EditorInput::CopySelectedRequest => {
                let ids: Vec<String> = self.items.iter()
                    .filter(|i| i.is_checked)
                    .map(|i| i.id.clone())
                    .collect();
                if !ids.is_empty() {
                    sender.output(EditorOutput::CopyItems(self.editor_type.clone(), ids)).ok();
                    self.visible = false;
                }
            }
            EditorInput::ShowContextMenu(index, x, y) => {
                let popover = &self.context_menu;
                let s_clone = sender.input_sender().clone();
                let box_ = self.create_context_menu_box(index, s_clone);
                
                popover.set_child(Some(&box_));
                
                if let Some(pos) = self.visible_indices.iter().position(|&idx| idx == index) {
                    if let Some(row) = self.list_box.row_at_index(pos as i32) {
                        if popover.parent().is_some() {
                            popover.unparent();
                        }
                        popover.set_parent(&row);
                        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
                        popover.popup();
                    }
                }
            }
        }
    }
}

fn build_menu_item(label: &str, destructive: bool) -> gtk::Button {
    let btn = gtk::Button::builder()
        .has_frame(false)
        .css_classes(vec!["flat", "menu-btn", if destructive { "destructive-action" } else { "" }])
        .build();
    let lbl = gtk::Label::builder()
        .label(label)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();
    btn.set_child(Some(&lbl));
    btn
}
