use crate::backend::auth::account::{refresh_all_accounts, verify_account_status, AccountStatus};
use crate::backend::auth::microsoft::{Account, AccountType};
use crate::config::Config;
use crate::frontend::app::AppMsg;
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug)]
pub struct AccountRow {
    account: Account,
    is_active: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for AccountRow {
    type Init = (Account, bool);
    type Input = ();
    type Output = AccountRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.account.username,
            #[watch]
            set_subtitle: &self.get_subtitle(),
            #[watch]
            set_activatable: !self.is_active,

            connect_activated[sender, uuid = self.account.uuid.clone()] => move |_| {
                sender.output(AccountRowOutput::Switch(uuid.clone())).ok();
            },

            add_prefix = &gtk::Image {
                #[watch]
                set_icon_name: if self.is_active { Some("object-select-symbolic") } else { Some("avatar-default-symbolic") },
                set_css_classes: &["accent"],
            },

            add_suffix = &gtk::MenuButton {
                set_icon_name: "view-more-symbolic",
                set_css_classes: &["flat", "circular"],
                set_valign: gtk::Align::Center,
                #[wrap(Some)]
                #[name = "row_popover"]
                set_popover = &gtk::Popover {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_css_classes: &["menu-box"],

                        // Copy Username
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-copy-symbolic"),
                                gtk::Label::new(Some("Copy Username")),
                            },
                            connect_clicked[username = self.account.username.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&username);
                            }
                        },

                        // Copy UUID
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-copy-symbolic"),
                                gtk::Label::new(Some("Copy UUID")),
                            },
                            connect_clicked[uuid = self.account.uuid.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&uuid);
                            }
                        },

                        gtk::Separator { set_css_classes: &["menu-separator"] },

                        // Remove
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn", "error"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-delete-symbolic"),
                                gtk::Label::new(Some("Remove Account")),
                            },
                            connect_clicked[sender, uuid = self.account.uuid.clone(), row_popover] => move |_| {
                                row_popover.popdown();
                                sender.output(AccountRowOutput::Remove(uuid.clone())).ok();
                            }
                        },
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            account: init.0,
            is_active: init.1,
        }
    }
}

impl AccountRow {
    fn get_subtitle(&self) -> String {
        let type_str = match self.account.account_type {
            AccountType::Microsoft => "Microsoft",
            AccountType::Offline => "Offline",
        };
        let status = verify_account_status(&self.account);
        let status_str = match &status {
            AccountStatus::Valid => "Ready",
            AccountStatus::ExpiringSoon => "Expiring Soon",
            AccountStatus::Expired => "Expired",
            AccountStatus::Unknown(e) => e.as_str(),
        };
        format!("{} ({})", status_str, type_str)
    }
}

#[derive(Debug)]
pub enum AccountRowOutput {
    Switch(String),
    Remove(String),
}

pub struct AccountView {
    config: Config,
    accounts: FactoryVecDeque<AccountRow>,
    visible: bool,
    refreshing: bool,
    refresh_message: String,
}

#[derive(Debug)]
pub enum AccountInput {
    UpdateConfig(Config),
    Open,
    ShowToast(String),
    SwitchAccount(String),
    RemoveAccount(String),
    RefreshAll,
    ResetRefreshing,
}

#[relm4::component(pub)]
impl Component for AccountView {
    type Init = Config;
    type Input = AccountInput;
    type Output = AppMsg;
    type CommandOutput = ();

    view! {
    adw::Bin {
        set_vexpand: true,

        #[wrap(Some)]
        #[name = "toast_overlay"]
        set_child = &adw::ToastOverlay {
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,

                adw::Clamp {
                    set_maximum_size: 600,
                    set_tightening_threshold: 400,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 16,

                            // ── Active account card ──
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_css_classes: &["card"],

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_margin_all: 16,
                                    set_spacing: 16,

                                    gtk::Image {
                                        set_icon_name: Some("avatar-default-symbolic"),
                                        set_pixel_size: 40,
                                        set_css_classes: &["dim-label"],
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_hexpand: true,
                                        set_spacing: 2,
                                        set_valign: gtk::Align::Center,

                                        gtk::Label {
                                            set_label: "Active Account",
                                            set_css_classes: &["heading"],
                                            set_halign: gtk::Align::Start,
                                        },
                                        gtk::Label {
                                            set_css_classes: &["title-2"],
                                            set_halign: gtk::Align::Start,
                                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                                            #[watch]
                                            set_label: &model.get_active_name(),
                                        },
                                    }
                                },
                            },

                            // ── Content area: Spinner or List ──
                            gtk::Stack {
                                add_named[Some("loading")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_halign: gtk::Align::Center,
                                    set_valign: gtk::Align::Center,
                                    set_spacing: 16,
                                    set_margin_top: 32,
                                    set_margin_bottom: 32,

                                    adw::Spinner {
                                        set_width_request: 48,
                                        set_height_request: 48,
                                    },

                                    gtk::Label {
                                        #[watch]
                                        set_label: &model.refresh_message,
                                        set_css_classes: &["dim-label"],
                                    }
                                },
                                add_named[Some("list")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 12,
                                    gtk::Label {
                                        set_label: "All Accounts",
                                        set_css_classes: &["heading"],
                                        set_halign: gtk::Align::Start,
                                        set_margin_start: 4,
                                        set_margin_top: 8,
                                    },

                                    #[local_ref]
                                    account_list -> gtk::ListBox {
                                        set_css_classes: &["boxed-list"],
                                        set_selection_mode: gtk::SelectionMode::None,
                                    }
                                },

                                #[watch]
                                set_visible_child_name: if model.refreshing { "loading" } else { "list" },
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        config: Config,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let accounts = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| match output {
                AccountRowOutput::Switch(uuid) => AccountInput::SwitchAccount(uuid),
                AccountRowOutput::Remove(uuid) => AccountInput::RemoveAccount(uuid),
            });

        let mut model = AccountView {
            config,
            accounts,
            visible: false,
            refreshing: false,
            refresh_message: String::new(),
        };

        model.populate_accounts();
        let account_list = model.accounts.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            AccountInput::UpdateConfig(config) => {
                self.config = config;
                self.populate_accounts();
            }
            AccountInput::Open => {
                self.visible = !self.visible;
            }
            AccountInput::ShowToast(text) => {
                let toast = adw::Toast::new(&text);
                if let Some(overlay) = self.find_toast_overlay(root.clone().upcast()) {
                    overlay.add_toast(toast);
                } else {
                    if let Some(win) = relm4::main_application().active_window() {
                        if let Some(overlay) = self.find_toast_overlay(win.upcast()) {
                            overlay.add_toast(toast);
                        }
                    }
                }
            }
            AccountInput::SwitchAccount(uuid) => {
                let _ = sender.output(AppMsg::SwitchAccount(uuid));
            }
            AccountInput::RemoveAccount(uuid) => {
                let _ = sender.output(AppMsg::RemoveAccount(uuid));
            }
            AccountInput::RefreshAll => {
                self.refreshing = true;
                self.refresh_message = "Refreshing all accounts…".to_string();

                let mut config_clone = self.config.clone();
                let sender_out = sender.output_sender().clone();
                let sender_in = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let _ = refresh_all_accounts(&mut config_clone);
                    let _ = sender_out.send(AppMsg::RefreshAccountsAll(config_clone));
                    let _ = sender_in.send(AccountInput::ResetRefreshing);
                });
            }
            AccountInput::ResetRefreshing => {
                self.refreshing = false;
            }
        }
    }
}

impl AccountView {
    fn find_toast_overlay(&self, start: gtk::Widget) -> Option<adw::ToastOverlay> {
        if let Some(overlay) = start.downcast_ref::<adw::ToastOverlay>() {
            return Some(overlay.clone());
        }

        let mut child = start.first_child();
        while let Some(c) = child {
            if let Some(found) = self.find_toast_overlay(c.clone()) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }

    fn populate_accounts(&mut self) {
        self.refreshing = false;
        let mut guard = self.accounts.guard();
        guard.clear();
        let active_uuid = self.config.active_account_uuid.clone();
        for acct in &self.config.accounts {
            let is_active = Some(acct.uuid.clone()) == active_uuid;
            guard.push_back((acct.clone(), is_active));
        }
    }

    fn get_active_name(&self) -> String {
        let active_uuid = self.config.active_account_uuid.clone();
        if let Some(active) = self
            .config
            .accounts
            .iter()
            .find(|a| Some(a.uuid.clone()) == active_uuid)
        {
            active.username.clone()
        } else {
            "No active account".to_string()
        }
    }
}
