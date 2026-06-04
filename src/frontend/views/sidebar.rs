use adw::prelude::*;
use relm4::prelude::*;
use std::collections::HashMap;

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum SidebarInput {
    /// Update the highlight on the navigation items.
    SetSelected(SidebarPage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarPage {
    Library,
    Discover,
    Accounts,
    Playtime,
    Assets,
    InstanceDetails,
}

#[derive(Debug)]
pub enum SidebarOutput {
    Navigate(SidebarPage),
}

// ─── Model ────────────────────────────────────────────────────────────────────

pub struct SidebarList {
    list_box: gtk::ListBox,
    selected: SidebarPage,
    rows: HashMap<SidebarPage, gtk::ListBoxRow>,
}

// ─── Component ────────────────────────────────────────────────────────────────

#[relm4::component(pub)]
impl SimpleComponent for SidebarList {
    type Init = ();
    type Input = SidebarInput;
    type Output = SidebarOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_propagate_natural_width: true,

            #[local_ref]
            list_box -> gtk::ListBox {
                add_css_class: "navigation-sidebar",
                set_selection_mode: gtk::SelectionMode::Single,
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_box = gtk::ListBox::new();
        list_box.add_css_class("navigation-sidebar");
        list_box.set_selection_mode(gtk::SelectionMode::Single);

        let mut model = SidebarList {
            list_box: list_box.clone(),
            selected: SidebarPage::Library,
            rows: HashMap::new(),
        };

        model.rebuild_list(&sender);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: SidebarInput, _sender: ComponentSender<Self>) {
        match msg {
            SidebarInput::SetSelected(page) => {
                self.selected = page;
                self.update_selection();
            }
        }
    }
}

// ─── Builder helpers ──────────────────────────────────────────────────────────

impl SidebarList {
    fn rebuild_list(&mut self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.rows.clear();

        // 1. Navigation Group
        let library_row =
            self.create_nav_row(SidebarPage::Library, "My Library", "folder-open-symbolic");
        self.list_box.append(&library_row);
        self.rows.insert(SidebarPage::Library, library_row);

        let discover_row =
            self.create_nav_row(SidebarPage::Discover, "Discover", "system-search-symbolic");
        self.list_box.append(&discover_row);
        self.rows.insert(SidebarPage::Discover, discover_row);

        // 2. Management Group
        let accounts_row = self.create_nav_row(
            SidebarPage::Accounts,
            "Accounts",
            "org.gnome.Settings-users-symbolic",
        );
        accounts_row.add_css_class("group-start");
        self.list_box.append(&accounts_row);
        self.rows.insert(SidebarPage::Accounts, accounts_row);

        let playtime_row = self.create_nav_row(
            SidebarPage::Playtime,
            "Playtime",
            "preferences-system-time-symbolic",
        );
        self.list_box.append(&playtime_row);
        self.rows.insert(SidebarPage::Playtime, playtime_row);

        let assets_row =
            self.create_nav_row(SidebarPage::Assets, "Assets", "folder-download-symbolic");
        self.list_box.append(&assets_row);
        self.rows.insert(SidebarPage::Assets, assets_row);

        // Set header function for separators
        self.list_box.set_header_func(|row, before| {
            if before.is_none() {
                row.set_header(None::<&gtk::Widget>);
                return;
            }

            if row.has_css_class("group-start") {
                let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
                sep.set_margin_top(0);
                sep.set_margin_bottom(0);
                sep.set_margin_start(0);
                sep.set_margin_end(0);
                row.set_header(Some(&sep));
            } else {
                row.set_header(None::<&gtk::Widget>);
            }
        });

        // Connect selection signal
        let s = sender.clone();
        let rows = self.rows.clone();
        self.list_box
            .connect_row_activated(move |_, activated_row| {
                for (page, row) in &rows {
                    if row == activated_row {
                        s.output(SidebarOutput::Navigate(*page)).ok();
                        break;
                    }
                }
            });

        self.update_selection();
    }

    fn create_nav_row(&self, _page: SidebarPage, title: &str, icon_name: &str) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::builder().activatable(true).build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_start(6)
            .margin_end(6)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(16)
            .build();

        let label = gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .build();

        content.append(&icon);
        content.append(&label);
        row.set_child(Some(&content));

        row
    }

    fn update_selection(&self) {
        if let Some(row) = self.rows.get(&self.selected) {
            self.list_box.select_row(Some(row));
        } else {
            self.list_box.select_row(None::<&gtk::ListBoxRow>);
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

// ─── Instance Sidebar ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum InstanceSidebarInput {
    SetActiveTab(String),
}

#[derive(Debug)]
pub enum InstanceSidebarOutput {
    SwitchTab(String),
}

pub struct InstanceSidebar {
    list_box: gtk::ListBox,
    selected: String,
    rows: HashMap<String, gtk::ListBoxRow>,
}

#[relm4::component(pub)]
impl SimpleComponent for InstanceSidebar {
    type Init = ();
    type Input = InstanceSidebarInput;
    type Output = InstanceSidebarOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_propagate_natural_width: true,

            #[local_ref]
            list_box -> gtk::ListBox {
                add_css_class: "navigation-sidebar",
                set_selection_mode: gtk::SelectionMode::Single,
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_box = gtk::ListBox::new();
        list_box.add_css_class("navigation-sidebar");
        list_box.set_selection_mode(gtk::SelectionMode::Single);

        let mut model = InstanceSidebar {
            list_box: list_box.clone(),
            selected: "summary".to_string(),
            rows: HashMap::new(),
        };

        model.rebuild_list(&sender);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: InstanceSidebarInput, _sender: ComponentSender<Self>) {
        match msg {
            InstanceSidebarInput::SetActiveTab(tab) => {
                self.selected = tab;
                self.update_selection();
            }
        }
    }
}

impl InstanceSidebar {
    fn rebuild_list(&mut self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.rows.clear();

        let tabs = [
            ("summary", "Summary", "go-home-symbolic"),
            ("editor", "Editor", "document-edit-symbolic"),
            ("settings", "Settings", "emblem-system-symbolic"),
            ("console", "Console", "utilities-terminal-symbolic"),
        ];

        for (id, title, icon) in tabs {
            let row = self.create_nav_row(title, icon);
            self.list_box.append(&row);
            self.rows.insert(id.to_string(), row);
        }

        let s = sender.clone();
        let rows = self.rows.clone();
        self.list_box
            .connect_row_activated(move |_, activated_row| {
                for (id, row) in &rows {
                    if row == activated_row {
                        s.output(InstanceSidebarOutput::SwitchTab(id.clone())).ok();
                        break;
                    }
                }
            });

        self.update_selection();
    }

    fn create_nav_row(&self, title: &str, icon_name: &str) -> gtk::ListBoxRow {
        let row = gtk::ListBoxRow::builder().activatable(true).build();

        let content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(12)
            .margin_start(6)
            .margin_end(6)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        let icon = gtk::Image::builder()
            .icon_name(icon_name)
            .pixel_size(16)
            .build();

        let label = gtk::Label::builder()
            .label(title)
            .halign(gtk::Align::Start)
            .build();

        content.append(&icon);
        content.append(&label);
        row.set_child(Some(&content));

        row
    }

    fn update_selection(&self) {
        if let Some(row) = self.rows.get(&self.selected) {
            self.list_box.select_row(Some(row));
        } else {
            self.list_box.select_row(None::<&gtk::ListBoxRow>);
        }
    }
}
