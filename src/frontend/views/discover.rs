use crate::backend::instance::modpack::{
    ModpackDetails, ModpackInfo, ModpackSource, ModpackVersionInfo, ModrinthSource,
};
use adw::prelude::*;
use gtk::gdk;
use gtk::glib;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::collections::HashMap;
use std::thread;

// Helper functions for UI formatting
fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn escape(text: &str) -> String {
    glib::markup_escape_text(text).to_string()
}

// ---------------------------------------------------------------------------
// 1. Modpack Card Factory Component (for lists/grids)
// ---------------------------------------------------------------------------
pub struct ModpackCard {
    pub info: ModpackInfo,
    pub icon: Option<gdk::Texture>,
}

#[derive(Debug)]
pub enum ModpackCardOutput {
    Clicked(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for ModpackCard {
    type Init = (ModpackInfo, Option<gdk::Texture>);
    type Input = ();
    type Output = ModpackCardOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::FlowBox;

    view! {
        #[root]
        gtk::Button {
            set_has_frame: false,
            set_css_classes: &["overview-card"],
            set_width_request: 220,
            set_height_request: 96,
            set_hexpand: false,
            set_vexpand: false,
            connect_clicked[sender, slug = self.info.slug.clone()] => move |_| {
                sender.output(ModpackCardOutput::Clicked(slug.clone())).ok();
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 12,
                set_margin_all: 10,

                gtk::Stack {
                    set_hhomogeneous: true,
                    set_vhomogeneous: true,
                    set_valign: gtk::Align::Center,
                    add_css_class: "overview-card-icon",

                    add_named[Some("icon")] = &gtk::Image {
                        #[watch]
                        set_paintable: self.icon.as_ref().map(|t| t as &gdk::Texture),
                        set_pixel_size: 56,
                    },
                    add_named[Some("fallback")] = &gtk::Image {
                        set_icon_name: Some("package-x-generic-symbolic"),
                        set_pixel_size: 56,
                    },

                    #[watch]
                    set_visible_child_name: if self.icon.is_some() { "icon" } else { "fallback" },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 4,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: &self.info.title,
                        set_css_classes: &["overview-card-title"],
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_halign: gtk::Align::Start,
                        set_max_width_chars: 18,
                    },

                    gtk::Label {
                        set_label: &self.info.description,
                        set_css_classes: &["overview-card-subtitle"],
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_wrap_mode: gtk::pango::WrapMode::Word,
                        set_lines: 2,
                        set_max_width_chars: 22,
                    },
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            info: init.0,
            icon: init.1,
        }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

// ---------------------------------------------------------------------------
// 2. Carousel Card Factory Component (for featured carousel)
// ---------------------------------------------------------------------------
pub struct CarouselCard {
    pub info: ModpackInfo,
    pub icon: Option<gdk::Texture>,
}

#[derive(Debug)]
pub enum CarouselCardOutput {
    Clicked(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for CarouselCard {
    type Init = (ModpackInfo, Option<gdk::Texture>);
    type Input = ();
    type Output = CarouselCardOutput;
    type CommandOutput = ();
    type ParentWidget = adw::Carousel;

    view! {
        #[root]
        #[name = "card_button"]
        gtk::Button {
            set_has_frame: false,
            set_css_classes: &["featured-carousel-item"],
            set_height_request: 200,
            set_hexpand: true,
            connect_clicked[sender, slug = self.info.slug.clone()] => move |_| {
                sender.output(CarouselCardOutput::Clicked(slug.clone())).ok();
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 28,
                set_margin_start: 60,
                set_margin_end: 60,
                set_margin_top: 20,
                set_margin_bottom: 20,

                gtk::Stack {
                    set_hhomogeneous: true,
                    set_vhomogeneous: true,
                    set_valign: gtk::Align::Center,

                    add_named[Some("icon")] = &gtk::Image {
                        #[watch]
                        set_paintable: self.icon.as_ref().map(|t| t as &gdk::Texture),
                        set_pixel_size: 96,
                    },
                    add_named[Some("fallback")] = &gtk::Image {
                        set_icon_name: Some("package-x-generic-symbolic"),
                        set_pixel_size: 96,
                    },
                    #[watch]
                    set_visible_child_name: if self.icon.is_some() { "icon" } else { "fallback" },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,

                    gtk::Label {
                        set_label: &self.info.title,
                        set_css_classes: &["title-3"],
                        set_halign: gtk::Align::Start,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_lines: 1,
                    },

                    gtk::Label {
                        set_label: &self.info.description,
                        set_css_classes: &["body"],
                        set_halign: gtk::Align::Start,
                        set_wrap: true,
                        set_wrap_mode: gtk::pango::WrapMode::Word,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                        set_lines: 2,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 16,

                        gtk::Label {
                            set_label: &format!("⬇ {} Downloads", format_downloads(self.info.downloads)),
                            set_css_classes: &["dim-label", "caption"],
                            set_valign: gtk::Align::Center,
                        },
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            info: init.0,
            icon: init.1,
        }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

// ---------------------------------------------------------------------------
// 3. Screenshot Card Factory Component (for details screenshot gallery)
// ---------------------------------------------------------------------------
pub struct ScreenshotCard {
    pub texture: gdk::Texture,
}

#[relm4::factory(pub)]
impl FactoryComponent for ScreenshotCard {
    type Init = gdk::Texture;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = adw::Carousel;

    view! {
        #[root]
        gtk::Picture {
            set_paintable: Some(&self.texture),
            set_can_shrink: true,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_hexpand: true,
            set_vexpand: true,
            add_css_class: "screenshot-img",
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { texture: init }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

// ---------------------------------------------------------------------------
// 3b. Version Select Row Factory Component (for dialog version list)
// ---------------------------------------------------------------------------
pub struct VersionSelectRow {
    pub version: ModpackVersionInfo,
    pub index: usize,
    pub is_selected: bool,
}

#[derive(Debug)]
pub enum VersionSelectRowOutput {
    Selected(usize),
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionSelectRow {
    type Init = (ModpackVersionInfo, usize, bool);
    type Input = ();
    type Output = VersionSelectRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &escape(&self.version.name),
            set_subtitle: &escape(&format!(
                "{} · {} · {}",
                self.version.version_number,
                self.version.loaders.join(", "),
                self.version.game_versions.join(", ")
            )),
            set_activatable: true,

            add_suffix = &gtk::Image {
                set_icon_name: Some("object-select-symbolic"),
                #[watch]
                set_visible: self.is_selected,
                set_valign: gtk::Align::Center,
            },

            connect_activated[sender, idx = self.index] => move |_| {
                sender.output(VersionSelectRowOutput::Selected(idx)).ok();
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            version: init.0,
            index: init.1,
            is_selected: init.2,
        }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

// ---------------------------------------------------------------------------
// 3c. Modpack Version Dialog Component
// ---------------------------------------------------------------------------
pub struct ModpackVersionDialog {
    versions: Vec<ModpackVersionInfo>,
    selected_idx: usize,
    visible: bool,
    version_rows: FactoryVecDeque<VersionSelectRow>,
}

#[derive(Debug)]
pub enum VersionDialogInput {
    Show(Vec<ModpackVersionInfo>, usize),
    Select(usize),
    Confirm,
    Close,
}

#[derive(Debug)]
pub enum VersionDialogOutput {
    Selected(usize),
}

#[relm4::component(pub)]
impl Component for ModpackVersionDialog {
    type Init = ();
    type Input = VersionDialogInput;
    type Output = VersionDialogOutput;
    type CommandOutput = ();

    view! {
        adw::Dialog {
            set_title: "Select Version",
            set_content_width: 500,
            set_content_height: 450,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Select Modpack Version",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 16,

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,
                        set_vscrollbar_policy: gtk::PolicyType::Automatic,

                        #[local_ref]
                        versions_list -> gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_css_classes: &["boxed-list"],
                        }
                    }
                },

                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "Cancel",
                        set_css_classes: &["pill"],
                        connect_clicked[sender] => move |_| {
                            sender.input(VersionDialogInput::Close);
                        }
                    },

                    gtk::Button {
                        set_label: "Use",
                        set_css_classes: &["pill", "suggested-action"],
                        connect_clicked[sender] => move |_| {
                            sender.input(VersionDialogInput::Confirm);
                        }
                    }
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let version_rows = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |out| match out {
                VersionSelectRowOutput::Selected(idx) => VersionDialogInput::Select(idx),
            });

        let model = ModpackVersionDialog {
            versions: Vec::new(),
            selected_idx: 0,
            visible: false,
            version_rows,
        };

        let versions_list = model.version_rows.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            VersionDialogInput::Show(versions, selected) => {
                self.versions = versions;
                self.selected_idx = selected;
                self.visible = true;

                let mut guard = self.version_rows.guard();
                guard.clear();
                for (idx, v) in self.versions.iter().enumerate() {
                    guard.push_back((v.clone(), idx, idx == self.selected_idx));
                }
            }
            VersionDialogInput::Select(idx) => {
                let old_idx = self.selected_idx;
                self.selected_idx = idx;
                let mut guard = self.version_rows.guard();
                if old_idx < guard.len() {
                    if let Some(row) = guard.get_mut(old_idx) {
                        row.is_selected = false;
                        guard.send(old_idx, ());
                    }
                }
                if idx < guard.len() {
                    if let Some(row) = guard.get_mut(idx) {
                        row.is_selected = true;
                        guard.send(idx, ());
                    }
                }
            }
            VersionDialogInput::Confirm => {
                sender.output(VersionDialogOutput::Selected(self.selected_idx)).ok();
                self.visible = false;
                root.close();
            }
            VersionDialogInput::Close => {
                self.visible = false;
                root.close();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Main Component Model & Controller
// ---------------------------------------------------------------------------
#[derive(Debug)]
pub enum DiscoverInput {
    LoadPopular,
    Search(String),
    SearchResultsReady(Result<Vec<ModpackInfo>, String>),
    LoadDetails(String),
    DetailsReady(Result<ModpackDetails, String>),
    VersionsReady(Result<Vec<ModpackVersionInfo>, String>),
    IconLoaded {
        url: String,
        width: i32,
        height: i32,
        rgba: Vec<u8>,
        average_color: Option<(u8, u8, u8)>,
    },
    ScreenshotLoaded {
        url: String,
        width: i32,
        height: i32,
        rgba: Vec<u8>,
    },
    SelectVersion(u32),
    CloseDetails,
    InstallClicked,
    ConfirmInstall(String),
    CancelInstall,
    PerformInstall,
    CarouselScroll(f64),
    OpenVersionDialog,
    OpenUrl(String),
}

#[derive(Debug)]
pub enum DiscoverOutput {
    InstallModpack(String, ModpackVersionInfo, String), // Name, VersionInfo, ProviderName
    DetailsOpened,
    DetailsClosed,
}

pub struct DiscoverView {
    search_query: String,
    loading: bool,
    error: Option<String>,

    popular_packs: FactoryVecDeque<ModpackCard>,
    carousel_packs: FactoryVecDeque<CarouselCard>,
    search_packs: FactoryVecDeque<ModpackCard>,
    screenshot_cards: FactoryVecDeque<ScreenshotCard>,

    // Details page state
    show_details: bool,
    pub(crate) selected_details: Option<ModpackDetails>,
    loading_details: bool,
    available_versions: Vec<ModpackVersionInfo>,
    selected_version_idx: usize,

    // Caches
    icon_cache: HashMap<String, gdk::Texture>,
    screenshot_cache: HashMap<String, gdk::Texture>,
    color_cache: HashMap<String, (u8, u8, u8)>,
    carousel_container: Option<gtk::Box>,

    // Installation popup state
    prompting_install: bool,
    install_instance_name: String,
    install_name_entry: Option<gtk::Entry>,

    version_dialog: Controller<ModpackVersionDialog>,
}

impl DiscoverView {
    #[allow(deprecated)]
    fn update_active_carousel_color(&self) {
        if let Some(ref container) = self.carousel_container {
            let mut r = 40;
            let mut g = 40;
            let mut b = 40;
            let mut has_color = false;

            let carousel = self.carousel_packs.widget();
            let page = carousel.position().round() as usize;
            if let Some(card) = self.carousel_packs.get(page) {
                if let Some(ref icon_url) = card.info.icon_url {
                    if let Some(&(cr, cg, cb)) = self.color_cache.get(icon_url) {
                        r = cr;
                        g = cg;
                        b = cb;
                        has_color = true;
                    }
                }
            }

            let provider = gtk::CssProvider::new();
            let css = if has_color {
                format!(
                    ".featured-carousel-container {{ background-color: rgba({}, {}, {}, 0.15); border: none; }}",
                    r, g, b
                )
            } else {
                ".featured-carousel-container { background-color: @card_bg_color; border: none; }".to_string()
            };
            provider.load_from_string(&css);
            container.style_context().add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for DiscoverView {
    type Init = ();
    type Input = DiscoverInput;
    type Output = DiscoverOutput;

    view! {
        #[name = "discover_stack"]
        gtk::Stack {
            set_vexpand: true,
            set_hexpand: true,
            set_transition_type: gtk::StackTransitionType::SlideLeftRight,
            set_transition_duration: 300,

            // ── Page 1: Browse View ──
            add_named[Some("browse")] = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                adw::Clamp {
                    set_maximum_size: 1024,
                    set_tightening_threshold: 800,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 20,
                        set_margin_all: 24,

                        // Search bar
                        gtk::SearchEntry {
                            set_placeholder_text: Some("Search Modrinth modpacks..."),
                            connect_search_changed[sender] => move |entry| {
                                sender.input(DiscoverInput::Search(entry.text().to_string()));
                            }
                        },

                        // Loading spinner
                        gtk::Spinner {
                            #[watch]
                            set_visible: model.loading,
                            set_spinning: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_width_request: 32,
                            set_height_request: 32,
                            set_margin_all: 24,
                        },

                        // Error page
                        adw::StatusPage {
                            #[watch]
                            set_visible: model.error.is_some() && !model.loading,
                            #[watch]
                            set_title: model.error.as_deref().unwrap_or("Error"),
                            set_description: Some("Could not fetch modpacks. Please check your internet connection and try again."),
                            set_icon_name: Some("network-offline-symbolic"),
                        },

                        // Empty results page
                        adw::StatusPage {
                            #[watch]
                            set_visible: !model.loading && model.error.is_none() && !model.search_query.is_empty() && model.search_packs.is_empty(),
                            set_title: "No Modpacks Found",
                            set_description: Some("Try refining your search terms."),
                            set_icon_name: Some("system-search-symbolic"),
                        },

                        // Layout 1: Featured Carousel + Popular Grid (When query is empty)
                        gtk::Box {
                            #[watch]
                            set_visible: !model.loading && model.error.is_none() && model.search_query.is_empty(),
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,
                            set_hexpand: true,

                            gtk::Label {
                                set_label: "Featured Modpacks",
                                set_css_classes: &["title-2"],
                                set_halign: gtk::Align::Start,
                            },

                            // Featured carousel with arrow button overlay navigation
                            #[name = "carousel_container"]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_css_classes: &["featured-carousel-container"],

                                gtk::Overlay {
                                    #[local_ref]
                                    featured_carousel -> adw::Carousel {
                                        set_hexpand: true,
                                        set_allow_scroll_wheel: false,
                                        connect_position_notify[sender] => move |carousel| {
                                            sender.input(DiscoverInput::CarouselScroll(carousel.position()));
                                        }
                                    },

                                    add_overlay = &gtk::Button {
                                        set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                        set_halign: gtk::Align::Start,
                                        set_valign: gtk::Align::Center,
                                        set_margin_start: 8,
                                        
                                        gtk::Image {
                                            set_icon_name: Some("go-previous-symbolic"),
                                            set_pixel_size: 16,
                                        },
                                        connect_clicked[featured_carousel] => move |_| {
                                            let page = featured_carousel.position().round() as u32;
                                            if page > 0 {
                                                let widget = featured_carousel.nth_page(page - 1);
                                                featured_carousel.scroll_to(&widget, true);
                                            }
                                        }
                                    },

                                    add_overlay = &gtk::Button {
                                        set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                        set_halign: gtk::Align::End,
                                        set_valign: gtk::Align::Center,
                                        set_margin_end: 8,
                                        
                                        gtk::Image {
                                            set_icon_name: Some("go-next-symbolic"),
                                            set_pixel_size: 16,
                                        },
                                        connect_clicked[featured_carousel] => move |_| {
                                            let page = featured_carousel.position().round() as u32;
                                            let n_pages = featured_carousel.n_pages();
                                            if page + 1 < n_pages {
                                                let widget = featured_carousel.nth_page(page + 1);
                                                featured_carousel.scroll_to(&widget, true);
                                            }
                                        }
                                    }
                                },
                            },

                            adw::CarouselIndicatorDots {
                                set_carousel: Some(&featured_carousel),
                                set_halign: gtk::Align::Center,
                                set_margin_top: 8,
                                set_margin_bottom: 8,
                            },

                            gtk::Label {
                                set_label: "Popular Modpacks",
                                set_css_classes: &["title-2"],
                                set_halign: gtk::Align::Start,
                                set_margin_top: 12,
                            },

                            #[local_ref]
                            popular_grid -> gtk::FlowBox {
                                set_valign: gtk::Align::Start,
                                set_hexpand: true,
                                set_homogeneous: true,
                                set_max_children_per_line: 3,
                                set_min_children_per_line: 1,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_column_spacing: 24,
                                set_row_spacing: 24,
                                add_css_class: "overview-grid",
                            }
                        },

                        // Layout 2: Search results grid (When query is NOT empty)
                        gtk::Box {
                            #[watch]
                            set_visible: !model.loading && model.error.is_none() && !model.search_query.is_empty() && !model.search_packs.is_empty(),
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 12,
                            set_hexpand: true,

                            gtk::Label {
                                set_label: "Search Results",
                                set_css_classes: &["title-2"],
                                set_halign: gtk::Align::Start,
                            },

                            #[local_ref]
                            search_grid -> gtk::FlowBox {
                                set_valign: gtk::Align::Start,
                                set_hexpand: true,
                                set_homogeneous: true,
                                set_max_children_per_line: 3,
                                set_min_children_per_line: 1,
                                set_selection_mode: gtk::SelectionMode::None,
                                set_column_spacing: 24,
                                set_row_spacing: 24,
                                add_css_class: "overview-grid",
                            }
                        }
                    }
                }
            },

            // ── Page 2: Details View (Full Width Stack) ──
            add_named[Some("details")] = &gtk::ScrolledWindow {
                set_vexpand: true,
                set_hexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_css_classes: &["background"],

                adw::Clamp {
                    set_maximum_size: 800,
                    set_tightening_threshold: 600,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 20,

                        // Loading spinner for details
                        gtk::Spinner {
                            #[watch]
                            set_visible: model.loading_details,
                            set_spinning: true,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_margin_all: 24,
                        },

                        // Details container
                        gtk::Box {
                            #[watch]
                            set_visible: !model.loading_details && model.selected_details.is_some(),
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 16,

                            // Header: Icon + Title + Stats
                            gtk::Box {
                                set_spacing: 16,

                                gtk::Stack {
                                    set_hhomogeneous: true,
                                    set_vhomogeneous: true,
                                    set_valign: gtk::Align::Center,

                                    add_named[Some("icon")] = &gtk::Image {
                                        #[watch]
                                        set_paintable: model.selected_details.as_ref()
                                            .and_then(|d| d.info.icon_url.as_ref())
                                            .and_then(|url| model.icon_cache.get(url))
                                            .map(|t| t as &gdk::Texture),
                                        set_pixel_size: 96,
                                        set_css_classes: &["icon-dropshadow"],
                                    },
                                    add_named[Some("fallback")] = &gtk::Image {
                                        set_icon_name: Some("package-x-generic-symbolic"),
                                        set_pixel_size: 96,
                                        set_css_classes: &["icon-dropshadow"],
                                    },
                                    #[watch]
                                    set_visible_child_name: if model.selected_details.as_ref()
                                        .and_then(|d| d.info.icon_url.as_ref())
                                        .and_then(|url| model.icon_cache.get(url))
                                        .is_some() { "icon" } else { "fallback" },
                                },

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,
                                    set_valign: gtk::Align::Center,
                                    set_hexpand: true,

                                    gtk::Label {
                                        #[watch]
                                        set_label: &escape(model.selected_details.as_ref().map(|d| d.info.title.as_str()).unwrap_or("")),
                                        set_css_classes: &["title-3"],
                                        set_halign: gtk::Align::Start,
                                        set_use_markup: true,
                                    },

                                    // Stats badges row
                                    gtk::Box {
                                        set_spacing: 8,
                                        set_halign: gtk::Align::Start,

                                        gtk::Label {
                                            #[watch]
                                            set_label: &format!("⬇ {}", format_downloads(model.selected_details.as_ref().map(|d| d.info.downloads).unwrap_or(0))),
                                            set_css_classes: &["pill-badge"],
                                        },
                                        gtk::Label {
                                            #[watch]
                                            set_label: &format!("♥ {}", format_downloads(model.selected_details.as_ref().map(|d| d.info.follows).unwrap_or(0))),
                                            set_css_classes: &["pill-badge"],
                                        },
                                    },
                                }
                            },

                            // Screenshot Gallery Carousel (nested, wrapped in a tight adw::Clamp)
                            adw::Clamp {
                                set_maximum_size: 480,
                                set_tightening_threshold: 400,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 6,
                                    #[watch]
                                    set_visible: !model.screenshot_cards.is_empty(),

                                    gtk::Overlay {
                                        #[local_ref]
                                        screenshot_carousel -> adw::Carousel {
                                            set_height_request: 240,
                                            set_hexpand: true,
                                            set_allow_scroll_wheel: false,
                                        },

                                        add_overlay = &gtk::Button {
                                            set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                            set_halign: gtk::Align::Start,
                                            set_valign: gtk::Align::Center,
                                            set_margin_start: 8,

                                            gtk::Image {
                                                set_icon_name: Some("go-previous-symbolic"),
                                                set_pixel_size: 16,
                                            },
                                            connect_clicked[screenshot_carousel] => move |_| {
                                                let page = screenshot_carousel.position().round() as u32;
                                                if page > 0 {
                                                    let widget = screenshot_carousel.nth_page(page - 1);
                                                    screenshot_carousel.scroll_to(&widget, true);
                                                }
                                            }
                                        },

                                        add_overlay = &gtk::Button {
                                            set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                            set_halign: gtk::Align::End,
                                            set_valign: gtk::Align::Center,
                                            set_margin_end: 8,

                                            gtk::Image {
                                                set_icon_name: Some("go-next-symbolic"),
                                                set_pixel_size: 16,
                                            },
                                            connect_clicked[screenshot_carousel] => move |_| {
                                                let page = screenshot_carousel.position().round() as u32;
                                                let n_pages = screenshot_carousel.n_pages();
                                                if page + 1 < n_pages {
                                                    let widget = screenshot_carousel.nth_page(page + 1);
                                                    screenshot_carousel.scroll_to(&widget, true);
                                                }
                                            }
                                        }
                                    },

                                    adw::CarouselIndicatorDots {
                                        set_carousel: Some(&screenshot_carousel),
                                        set_halign: gtk::Align::Center,
                                    }
                                },
                            },

                            // Description Box
                            adw::PreferencesGroup {
                                set_title: "Description",
                                gtk::Label {
                                    #[watch]
                                    set_label: &escape(model.selected_details.as_ref().map(|d| d.info.description.as_str()).unwrap_or("")),
                                    set_wrap: true,
                                    set_halign: gtk::Align::Fill,
                                    set_xalign: 0.0,
                                    set_use_markup: true,
                                    set_margin_all: 10,
                                }
                            },

                            // Information Group
                            adw::PreferencesGroup {
                                set_title: "Information",

                                adw::ActionRow {
                                    set_title: "Minecraft Versions",
                                    add_prefix = &gtk::Image { set_icon_name: Some("computer-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref()
                                        .map(|d| d.game_versions.join(", "))
                                        .unwrap_or_default(),
                                    set_subtitle_lines: 2,
                                },
                                adw::ActionRow {
                                    set_title: "Supported Loaders",
                                    add_prefix = &gtk::Image { set_icon_name: Some("system-run-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref()
                                        .map(|d| d.loaders.join(", "))
                                        .unwrap_or_default(),
                                    set_subtitle_lines: 1,
                                },
                                adw::ActionRow {
                                    set_title: "Categories",
                                    add_prefix = &gtk::Image { set_icon_name: Some("preferences-desktop-apps-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref()
                                        .map(|d| d.info.categories.join(", "))
                                        .unwrap_or_default(),
                                    set_subtitle_lines: 2,
                                },
                            },

                            // Links section
                            adw::PreferencesGroup {
                                set_title: "Links",
                                #[watch]
                                set_visible: model.selected_details.is_some(),

                                adw::ActionRow {
                                    set_title: "Modrinth Page",
                                    add_prefix = &gtk::Image { set_icon_name: Some("web-browser-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref().map(|d| format!("https://modrinth.com/modpack/{}", d.info.id)).unwrap_or_default(),
                                    set_subtitle_lines: 1,
                                    set_activatable: true,
                                    connect_activated[sender] => move |row| {
                                        if let Some(subtitle) = row.subtitle() {
                                            let url = subtitle.to_string();
                                            if !url.is_empty() {
                                                sender.input(DiscoverInput::OpenUrl(url));
                                            }
                                        }
                                    }
                                },
                                adw::ActionRow {
                                    set_title: "Source Code",
                                    add_prefix = &gtk::Image { set_icon_name: Some("text-editor-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_visible: model.selected_details.as_ref().and_then(|d| d.source_url.as_ref()).is_some(),
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref().and_then(|d| d.source_url.clone()).unwrap_or_default(),
                                    set_subtitle_lines: 1,
                                    set_activatable: true,
                                    connect_activated[sender] => move |row| {
                                        if let Some(subtitle) = row.subtitle() {
                                            let url = subtitle.to_string();
                                            if !url.is_empty() {
                                                sender.input(DiscoverInput::OpenUrl(url));
                                            }
                                        }
                                    }
                                },
                                adw::ActionRow {
                                    set_title: "Wiki / Docs",
                                    add_prefix = &gtk::Image { set_icon_name: Some("accessories-dictionary-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_visible: model.selected_details.as_ref().and_then(|d| d.wiki_url.as_ref()).is_some(),
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref().and_then(|d| d.wiki_url.clone()).unwrap_or_default(),
                                    set_subtitle_lines: 1,
                                    set_activatable: true,
                                    connect_activated[sender] => move |row| {
                                        if let Some(subtitle) = row.subtitle() {
                                            let url = subtitle.to_string();
                                            if !url.is_empty() {
                                                sender.input(DiscoverInput::OpenUrl(url));
                                            }
                                        }
                                    }
                                },
                                adw::ActionRow {
                                    set_title: "Discord",
                                    add_prefix = &gtk::Image { set_icon_name: Some("chat-message-new-symbolic"), set_pixel_size: 16 },
                                    #[watch]
                                    set_visible: model.selected_details.as_ref().and_then(|d| d.discord_url.as_ref()).is_some(),
                                    #[watch]
                                    set_subtitle: &model.selected_details.as_ref().and_then(|d| d.discord_url.clone()).unwrap_or_default(),
                                    set_subtitle_lines: 1,
                                    set_activatable: true,
                                    connect_activated[sender] => move |row| {
                                        if let Some(subtitle) = row.subtitle() {
                                            let url = subtitle.to_string();
                                            if !url.is_empty() {
                                                sender.input(DiscoverInput::OpenUrl(url));
                                            }
                                        }
                                    }
                                },
                            },

                            // Installation / Version selector
                            adw::PreferencesGroup {
                                set_title: "Installation",

                                adw::ActionRow {
                                    set_title: "Select Version",
                                    #[watch]
                                    set_subtitle: &if let Some(v) = model.available_versions.get(model.selected_version_idx) {
                                        format!("{} ({})", v.name, v.version_number)
                                    } else {
                                        "No version selected".to_string()
                                    },
                                    set_activatable: true,
                                    connect_activated[sender] => move |_| {
                                        sender.input(DiscoverInput::OpenVersionDialog);
                                    },
                                    add_suffix = &gtk::Button {
                                        set_icon_name: "edit-symbolic",
                                        set_css_classes: &["flat", "circular"],
                                        set_tooltip_text: Some("Select Version"),
                                        set_valign: gtk::Align::Center,
                                        connect_clicked[sender] => move |_| {
                                            sender.input(DiscoverInput::OpenVersionDialog);
                                        }
                                    }
                                }
                            },

                            // Install action card
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 8,

                                gtk::Box {
                                    #[watch]
                                    set_visible: !model.prompting_install,
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 12,

                                    gtk::Button {
                                        set_label: "Install Modpack",
                                        set_css_classes: &["suggested-action", "pill"],
                                        set_hexpand: true,
                                        set_height_request: 40,
                                        connect_clicked => DiscoverInput::InstallClicked,
                                    }
                                },

                                gtk::Box {
                                    #[watch]
                                    set_visible: model.prompting_install,
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 8,
                                    set_css_classes: &["card"],
                                    set_margin_all: 4,

                                    gtk::Label {
                                        set_label: "Choose Instance Name:",
                                        set_halign: gtk::Align::Start,
                                        set_css_classes: &["dim-label", "caption"],
                                        set_margin_start: 12,
                                        set_margin_top: 8,
                                    },

                                    #[name = "install_name_entry"]
                                    gtk::Entry {
                                        set_placeholder_text: Some("My Modpack Instance"),
                                        set_margin_start: 12,
                                        set_margin_end: 12,
                                        connect_changed[sender] => move |entry| {
                                            sender.input(DiscoverInput::ConfirmInstall(entry.text().to_string()));
                                        },
                                        connect_activate => DiscoverInput::PerformInstall,
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 12,
                                        set_margin_all: 12,

                                        gtk::Button {
                                            set_label: "Confirm & Install",
                                            set_css_classes: &["suggested-action", "pill"],
                                            set_hexpand: true,
                                            #[watch]
                                            set_sensitive: !model.install_instance_name.trim().is_empty(),
                                            connect_clicked => DiscoverInput::PerformInstall,
                                        },

                                        gtk::Button {
                                            set_label: "Cancel",
                                            set_css_classes: &["pill"],
                                            set_hexpand: true,
                                            connect_clicked => DiscoverInput::CancelInstall,
                                        }
                                    }
                                }
                            },

                            // License footer (centered)
                            gtk::Label {
                                #[watch]
                                set_label: &{
                                    model.selected_details.as_ref()
                                        .and_then(|d| d.license_name.clone())
                                        .map(|l| format!("Licensed under {}", l))
                                        .unwrap_or_default()
                                },
                                #[watch]
                                set_visible: model.selected_details.as_ref().and_then(|d| d.license_name.as_ref()).is_some(),
                                set_halign: gtk::Align::Center,
                                set_css_classes: &["dim-label", "caption"],
                                set_margin_top: 8,
                                set_margin_bottom: 16,
                            }
                        }
                    }
                }
            },
            #[watch]
            set_visible_child_name: if model.show_details { "details" } else { "browse" },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let popular_packs = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.input_sender(), |output| match output {
                ModpackCardOutput::Clicked(slug) => DiscoverInput::LoadDetails(slug),
            });

        let carousel_packs = FactoryVecDeque::builder()
            .launch(adw::Carousel::new())
            .forward(sender.input_sender(), |output| match output {
                CarouselCardOutput::Clicked(slug) => DiscoverInput::LoadDetails(slug),
            });

        let search_packs = FactoryVecDeque::builder()
            .launch(gtk::FlowBox::new())
            .forward(sender.input_sender(), |output| match output {
                ModpackCardOutput::Clicked(slug) => DiscoverInput::LoadDetails(slug),
            });

        let screenshot_cards = FactoryVecDeque::builder()
            .launch(adw::Carousel::new())
            .forward(sender.input_sender(), |_| DiscoverInput::CloseDetails); // Dummy forward

        let version_dialog = ModpackVersionDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                VersionDialogOutput::Selected(idx) => DiscoverInput::SelectVersion(idx as u32),
            });

        let mut model = DiscoverView {
            search_query: String::new(),
            loading: false,
            error: None,

            popular_packs,
            carousel_packs,
            search_packs,
            screenshot_cards,

            show_details: false,
            selected_details: None,
            loading_details: false,
            available_versions: Vec::new(),
            selected_version_idx: 0,

            icon_cache: HashMap::new(),
            screenshot_cache: HashMap::new(),
            color_cache: HashMap::new(),
            carousel_container: None,

            prompting_install: false,
            install_instance_name: String::new(),
            install_name_entry: None,
            version_dialog,
        };

        let popular_grid = model.popular_packs.widget();
        let featured_carousel = model.carousel_packs.widget();
        let search_grid = model.search_packs.widget();
        let screenshot_carousel = model.screenshot_cards.widget();

        let widgets = view_output!();
        model.carousel_container = Some(widgets.carousel_container.clone());
        model.install_name_entry = Some(widgets.install_name_entry.clone());

        // Trigger loading of popular modpacks on startup
        sender.input(DiscoverInput::LoadPopular);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DiscoverInput::LoadPopular => {
                self.loading = true;
                self.error = None;
                let sender_clone = sender.input_sender().clone();
                thread::spawn(move || {
                    let source = ModrinthSource;
                    let result = source.get_popular(25, 0);
                    sender_clone.send(DiscoverInput::SearchResultsReady(result)).ok();
                });
            }
            DiscoverInput::Search(query) => {
                self.search_query = query.trim().to_string();
                self.loading = true;
                self.error = None;
                
                let query_clone = self.search_query.clone();
                let sender_clone = sender.input_sender().clone();
                thread::spawn(move || {
                    let source = ModrinthSource;
                    let result = source.search(&query_clone, 25, 0, None, None);
                    sender_clone.send(DiscoverInput::SearchResultsReady(result)).ok();
                });
            }
            DiscoverInput::SearchResultsReady(result) => {
                self.loading = false;
                match result {
                    Ok(packs) => {
                        self.error = None;
                        if self.search_query.is_empty() {
                            // Populate carousel with the first 5, popular_packs with the rest
                            let mut carousel_guard = self.carousel_packs.guard();
                            let mut popular_guard = self.popular_packs.guard();
                            carousel_guard.clear();
                            popular_guard.clear();

                            for (idx, pack) in packs.into_iter().enumerate() {
                                let icon_tex = pack.icon_url.as_ref().and_then(|url| self.icon_cache.get(url)).cloned();
                                if idx < 5 {
                                    carousel_guard.push_back((pack.clone(), icon_tex.clone()));
                                } else {
                                    popular_guard.push_back((pack.clone(), icon_tex.clone()));
                                }

                                // Load icon if not cached
                                if let Some(ref icon_url) = pack.icon_url {
                                    if !self.icon_cache.contains_key(icon_url) {
                                        fetch_icon(icon_url.clone(), sender.input_sender().clone());
                                    }
                                }
                            }
                        } else {
                            // Search results grid
                            let mut search_guard = self.search_packs.guard();
                            search_guard.clear();

                            for pack in packs {
                                let icon_tex = pack.icon_url.as_ref().and_then(|url| self.icon_cache.get(url)).cloned();
                                search_guard.push_back((pack.clone(), icon_tex.clone()));

                                // Load icon if not cached
                                if let Some(ref icon_url) = pack.icon_url {
                                    if !self.icon_cache.contains_key(icon_url) {
                                        fetch_icon(icon_url.clone(), sender.input_sender().clone());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
            }
            DiscoverInput::LoadDetails(slug) => {
                self.show_details = true;
                self.loading_details = true;
                self.selected_details = None;
                self.prompting_install = false;
                self.install_instance_name.clear();
                
                // Clear screenshot carousel
                self.screenshot_cards.guard().clear();

                // Notify parent component that details panel has opened
                sender.output(DiscoverOutput::DetailsOpened).ok();

                let slug_clone = slug.clone();
                let sender_clone = sender.input_sender().clone();
                thread::spawn(move || {
                    let source = ModrinthSource;
                    let details_res = source.get_details(&slug_clone);
                    sender_clone.send(DiscoverInput::DetailsReady(details_res)).ok();

                    let versions_res = source.get_versions(&slug_clone);
                    sender_clone.send(DiscoverInput::VersionsReady(versions_res)).ok();
                });
            }
            DiscoverInput::DetailsReady(result) => {
                self.loading_details = false;
                match result {
                    Ok(details) => {
                        // Pre-populate instance name
                        self.install_instance_name = details.info.title.clone();
                        // Set entry text imperatively (not via #[watch]) to avoid
                        // an infinite set_text → changed → ConfirmInstall → set_text loop
                        if let Some(ref entry) = self.install_name_entry {
                            entry.set_text(&self.install_instance_name);
                        }
                        
                        // Load screenshots
                        let mut guard = self.screenshot_cards.guard();
                        for url in &details.screenshots {
                            if let Some(tex) = self.screenshot_cache.get(url) {
                                guard.push_back(tex.clone());
                            } else {
                                fetch_screenshot(url.clone(), sender.input_sender().clone());
                            }
                        }

                        // Load details icon if not in cache
                        if let Some(ref icon_url) = details.info.icon_url {
                            if !self.icon_cache.contains_key(icon_url) {
                                fetch_icon(icon_url.clone(), sender.input_sender().clone());
                            }
                        }

                        self.selected_details = Some(details);
                    }
                    Err(e) => {
                        self.error = Some(e);
                        self.show_details = false;
                        sender.output(DiscoverOutput::DetailsClosed).ok();
                    }
                }
            }
            DiscoverInput::VersionsReady(result) => {
                match result {
                    Ok(versions) => {
                        self.available_versions = versions;
                        self.selected_version_idx = 0;
                    }
                    Err(e) => {
                        eprintln!("[discover] Failed to load versions: {}", e);
                    }
                }
            }
            DiscoverInput::IconLoaded {
                url,
                width,
                height,
                rgba,
                average_color,
            } => {
                let gbytes = glib::Bytes::from(&rgba);
                let texture = gdk::MemoryTexture::new(
                    width,
                    height,
                    gdk::MemoryFormat::R8g8b8a8,
                    &gbytes,
                    (width * 4) as usize,
                );
                let texture: gdk::Texture = texture.upcast();

                self.icon_cache.insert(url.clone(), texture.clone());

                if let Some(color) = average_color {
                    self.color_cache.insert(url.clone(), color);
                }

                // Update popular packs
                let mut popular_guard = self.popular_packs.guard();
                for i in 0..popular_guard.len() {
                    if let Some(row) = popular_guard.get_mut(i) {
                        if row.info.icon_url.as_ref() == Some(&url) {
                            row.icon = Some(texture.clone());
                            popular_guard.send(i, ());
                        }
                    }
                }

                // Update carousel packs
                let mut carousel_guard = self.carousel_packs.guard();
                for i in 0..carousel_guard.len() {
                    if let Some(row) = carousel_guard.get_mut(i) {
                        if row.info.icon_url.as_ref() == Some(&url) {
                            row.icon = Some(texture.clone());
                            carousel_guard.send(i, ());
                        }
                    }
                }

                // Update search packs
                let mut search_guard = self.search_packs.guard();
                for i in 0..search_guard.len() {
                    if let Some(row) = search_guard.get_mut(i) {
                        if row.info.icon_url.as_ref() == Some(&url) {
                            row.icon = Some(texture.clone());
                            search_guard.send(i, ());
                        }
                    }
                }
                drop(popular_guard);
                drop(carousel_guard);
                drop(search_guard);
                #[allow(deprecated)]
                self.update_active_carousel_color();
            }
            DiscoverInput::ScreenshotLoaded {
                url,
                width,
                height,
                rgba,
            } => {
                let gbytes = glib::Bytes::from(&rgba);
                let texture = gdk::MemoryTexture::new(
                    width,
                    height,
                    gdk::MemoryFormat::R8g8b8a8,
                    &gbytes,
                    (width * 4) as usize,
                );
                let texture: gdk::Texture = texture.upcast();

                self.screenshot_cache.insert(url.clone(), texture.clone());
                
                // If this screenshot is part of the currently selected details, append to the carousel
                if let Some(ref details) = self.selected_details {
                    if details.screenshots.contains(&url) {
                        self.screenshot_cards.guard().push_back(texture);
                    }
                }
            }
            DiscoverInput::SelectVersion(idx) => {
                let idx_usize = idx as usize;
                if idx_usize < self.available_versions.len() {
                    self.selected_version_idx = idx_usize;
                }
            }
            DiscoverInput::CloseDetails => {
                self.show_details = false;
                self.prompting_install = false;
                sender.output(DiscoverOutput::DetailsClosed).ok();
            }
            DiscoverInput::InstallClicked => {
                self.prompting_install = true;
            }
            DiscoverInput::ConfirmInstall(name) => {
                self.install_instance_name = name;
            }
            DiscoverInput::CancelInstall => {
                self.prompting_install = false;
            }
            DiscoverInput::PerformInstall => {
                if let Some(version) = self.available_versions.get(self.selected_version_idx) {
                    sender.output(DiscoverOutput::InstallModpack(
                        self.install_instance_name.trim().to_string(),
                        version.clone(),
                        "Modrinth".to_string()
                    )).unwrap();
                    
                    self.show_details = false;
                    self.prompting_install = false;
                    sender.output(DiscoverOutput::DetailsClosed).ok();
                }
            }
            DiscoverInput::CarouselScroll(_pos) => {
                self.update_active_carousel_color();
            }
            DiscoverInput::OpenVersionDialog => {
                self.version_dialog.emit(VersionDialogInput::Show(self.available_versions.clone(), self.selected_version_idx));
                let parent = relm4::main_application().active_window();
                self.version_dialog.widget().present(parent.as_ref());
            }
            DiscoverInput::OpenUrl(url) => {
                crate::frontend::utils::open_url(&url);
            }
        }
    }
}

// Background network helpers for images (sends decoded data back to main thread)
fn fetch_icon(url: String, sender: relm4::Sender<DiscoverInput>) {
    thread::spawn(move || {
        use crate::backend::instance::modpack::HTTP_CLIENT;
        if let Ok(res) = HTTP_CLIENT.get(&url).send() {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes() {
                    let bytes_vec = bytes.to_vec();
                    if let Ok(img) = image::load_from_memory(&bytes_vec) {
                        let width = img.width() as i32;
                        let height = img.height() as i32;
                        let rgba_img = img.to_rgba8();
                        
                        // Calculate average color
                        let mut r_sum = 0u64;
                        let mut g_sum = 0u64;
                        let mut b_sum = 0u64;
                        let mut count = 0u64;
                        for pixel in rgba_img.pixels() {
                            if pixel[3] > 30 {
                                r_sum += pixel[0] as u64;
                                g_sum += pixel[1] as u64;
                                b_sum += pixel[2] as u64;
                                count += 1;
                            }
                        }
                        let average_color = if count > 0 {
                            Some((
                                (r_sum / count) as u8,
                                (g_sum / count) as u8,
                                (b_sum / count) as u8,
                            ))
                        } else {
                            None
                        };

                        let _ = sender.send(DiscoverInput::IconLoaded {
                            url,
                            width,
                            height,
                            rgba: rgba_img.into_raw(),
                            average_color,
                        });
                    }
                }
            }
        }
    });
}

fn fetch_screenshot(url: String, sender: relm4::Sender<DiscoverInput>) {
    thread::spawn(move || {
        use crate::backend::instance::modpack::HTTP_CLIENT;
        if let Ok(res) = HTTP_CLIENT.get(&url).send() {
            if res.status().is_success() {
                if let Ok(bytes) = res.bytes() {
                    let bytes_vec = bytes.to_vec();
                    if let Ok(img) = image::load_from_memory(&bytes_vec) {
                        let width = img.width() as i32;
                        let height = img.height() as i32;
                        let _ = sender.send(DiscoverInput::ScreenshotLoaded {
                            url,
                            width,
                            height,
                            rgba: img.to_rgba8().into_raw(),
                        });
                    }
                }
            }
        }
    });
}
