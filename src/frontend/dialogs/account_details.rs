use crate::backend::auth::account::{verify_account_status, AccountStatus};
use crate::backend::auth::avatar::{fetch_and_cache_avatar, get_avatar_cache_path};
use crate::backend::auth::microsoft::{
    get_minecraft_profile, select_minecraft_cape, Account, AccountType, McCape,
};
use adw::prelude::*;
use gtk::gdk;
use relm4::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AccountDetailsDialog {
    account: Option<Account>,
    avatar_texture: Option<gdk::Texture>,
    capes: Vec<McCape>,
    selected_cape_idx: u32,
    cape_model: gtk::StringList,
    updating_cape: bool,
    cape_status: String,
}

#[derive(Debug)]
pub enum AccountDetailsInput {
    Show(Account),
    ProfileLoaded(Option<gdk::Texture>, Vec<McCape>, u32),
    SelectCape(u32),
    CapeUpdated(Result<(), String>),
}

#[relm4::component(pub)]
impl SimpleComponent for AccountDetailsDialog {
    type Init = ();
    type Input = AccountDetailsInput;
    type Output = ();

    view! {
        adw::Dialog {
            set_title: "Account Details",
            set_content_width: 480,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Account Details",
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_height: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 16,

                        // Header Card: Avatar Head & Username
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 16,
                            set_css_classes: &["card"],
                            set_margin_all: 0,

                            gtk::Box {
                                set_margin_all: 16,
                                set_spacing: 16,
                                set_valign: gtk::Align::Center,

                                gtk::Image {
                                    set_pixel_size: 64,
                                    #[watch]
                                    set_paintable: model.avatar_texture.as_ref().map(|t| t as &gdk::Texture),
                                    #[watch]
                                    set_icon_name: if model.avatar_texture.is_none() { Some("avatar-default-symbolic") } else { None },
                                    set_css_classes: &["accent"],
                                },

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_valign: gtk::Align::Center,
                                    set_spacing: 4,

                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_css_classes: &["title-2"],
                                        #[watch]
                                        set_label: model.account.as_ref().map(|a| a.username.as_str()).unwrap_or("Account Details"),
                                    },
                                    gtk::Label {
                                        set_halign: gtk::Align::Start,
                                        set_css_classes: &["dim-label", "caption"],
                                        #[watch]
                                        set_label: &model.get_account_type_str(),
                                    },
                                }
                            }
                        },

                        // Account Metadata Preferences Group
                        adw::PreferencesGroup {
                            set_title: "Account Overview",

                            // Account Type Row
                            adw::ActionRow {
                                set_title: "Account Type",
                                set_subtitle: "Authentication provider",
                                add_suffix = &gtk::Label {
                                    set_css_classes: &["dim-label"],
                                    #[watch]
                                    set_label: &model.get_account_type_str(),
                                }
                            },

                            // Status Row
                            adw::ActionRow {
                                set_title: "Status",
                                set_subtitle: "Authentication token validity",
                                add_suffix = &gtk::Label {
                                    set_css_classes: &["dim-label"],
                                    #[watch]
                                    set_label: &model.get_status_str(),
                                }
                            },

                            // UUID Row with copy button
                            adw::ActionRow {
                                set_title: "UUID",
                                #[watch]
                                set_subtitle: model.account.as_ref().map(|a| a.uuid.as_str()).unwrap_or("-"),
                                add_suffix = &gtk::Button {
                                    set_icon_name: "edit-copy-symbolic",
                                    set_valign: gtk::Align::Center,
                                    set_css_classes: &["flat", "circular"],
                                    set_tooltip_text: Some("Copy UUID"),
                                    connect_clicked[uuid = model.account.as_ref().map(|a| a.uuid.clone())] => move |btn| {
                                        if let Some(ref u) = uuid {
                                            btn.display().clipboard().set_text(u);
                                        }
                                    }
                                }
                            },

                            // Token Expiration (if Microsoft)
                            adw::ActionRow {
                                set_title: "Token Expiration",
                                set_subtitle: "Session expiration timeframe",
                                #[watch]
                                set_visible: model.account.as_ref().map(|a| a.account_type == AccountType::Microsoft).unwrap_or(false),
                                add_suffix = &gtk::Label {
                                    set_css_classes: &["dim-label"],
                                    #[watch]
                                    set_label: &model.get_expiry_str(),
                                }
                            },
                        },

                        // Cosmetics & Capes Group
                        adw::PreferencesGroup {
                            set_title: "Player Cosmetics &amp; Cape",

                            // Clean AdwComboRow styled matching libadwaita preferences
                            adw::ComboRow {
                                set_title: "Equipped Cape",
                                #[watch]
                                set_subtitle: &model.cape_status,
                                set_model: Some(&model.cape_model),
                                #[watch]
                                set_selected: model.selected_cape_idx,
                                #[watch]
                                set_sensitive: !model.updating_cape && model.account.as_ref().map(|a| a.account_type == AccountType::Microsoft).unwrap_or(false),

                                connect_selected_notify[sender] => move |row| {
                                    sender.input(AccountDetailsInput::SelectCape(row.selected()));
                                },
                            },
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let cape_model = gtk::StringList::new(&["None"]);
        let model = AccountDetailsDialog {
            account: None,
            avatar_texture: None,
            capes: Vec::new(),
            selected_cape_idx: 0,
            cape_model,
            updating_cape: false,
            cape_status: "No capes available".to_string(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AccountDetailsInput::Show(account) => {
                self.account = Some(account.clone());
                self.avatar_texture = None;
                self.capes.clear();
                self.selected_cape_idx = 0;
                self.updating_cape = false;
                self.cape_model.splice(0, self.cape_model.n_items(), &["None"]);

                let cache_path = get_avatar_cache_path(&account.uuid);
                if cache_path.exists() {
                    self.avatar_texture = gdk::Texture::from_filename(&cache_path).ok();
                }

                if account.account_type == AccountType::Microsoft {
                    self.cape_status = "Loading capes…".to_string();
                    let sender_in = sender.input_sender().clone();
                    let acct_clone = account.clone();

                    crate::backend::core::tasks::spawn_io(move || {
                        let tex = if !cache_path.exists() {
                            let _ = fetch_and_cache_avatar(&acct_clone.uuid);
                            gdk::Texture::from_filename(&cache_path).ok()
                        } else {
                            gdk::Texture::from_filename(&cache_path).ok()
                        };

                        let mut capes = Vec::new();
                        let mut active_idx = 0u32;
                        if let Ok(profile) = get_minecraft_profile(&acct_clone.access_token) {
                            capes = profile.capes;
                            for (idx, cape) in capes.iter().enumerate() {
                                if cape.state == "ACTIVE" {
                                    active_idx = (idx + 1) as u32;
                                }
                            }
                        }

                        let _ = sender_in.send(AccountDetailsInput::ProfileLoaded(tex, capes, active_idx));
                    });
                } else {
                    self.cape_status = "Not available for offline accounts".to_string();
                    let acct_clone = account;
                    let sender_in = sender.input_sender().clone();
                    crate::backend::core::tasks::spawn_io(move || {
                        if !cache_path.exists() {
                            let _ = fetch_and_cache_avatar(&acct_clone.uuid);
                        }
                        let tex = gdk::Texture::from_filename(&cache_path).ok();
                        let _ = sender_in.send(AccountDetailsInput::ProfileLoaded(tex, Vec::new(), 0));
                    });
                }
            }

            AccountDetailsInput::ProfileLoaded(tex, capes, active_idx) => {
                if let Some(t) = tex {
                    self.avatar_texture = Some(t);
                }
                self.capes = capes;
                self.selected_cape_idx = active_idx;

                let mut labels = vec!["None".to_string()];
                for cape in &self.capes {
                    let label = cape
                        .alias
                        .clone()
                        .unwrap_or_else(|| cape.id.clone());
                    labels.push(label);
                }

                let str_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
                self.cape_model.splice(0, self.cape_model.n_items(), &str_refs);

                if self.capes.is_empty() {
                    self.cape_status = "No capes owned".to_string();
                } else {
                    self.cape_status = format!("{} cape(s) available", self.capes.len());
                }
            }

            AccountDetailsInput::SelectCape(idx) => {
                if idx == self.selected_cape_idx || self.updating_cape {
                    return;
                }

                if let Some(ref acct) = self.account {
                    if acct.account_type == AccountType::Microsoft {
                        self.updating_cape = true;
                        self.selected_cape_idx = idx;
                        self.cape_status = "Updating cape…".to_string();

                        let target_cape_id = if idx == 0 {
                            None
                        } else {
                            self.capes.get((idx - 1) as usize).map(|c| c.id.as_str())
                        }
                        .map(|s| s.to_string());

                        let token = acct.access_token.clone();
                        let sender_in = sender.input_sender().clone();

                        crate::backend::core::tasks::spawn_io(move || {
                            let res = select_minecraft_cape(&token, target_cape_id.as_deref())
                                .map_err(|e| e.to_string());
                            let _ = sender_in.send(AccountDetailsInput::CapeUpdated(res));
                        });
                    }
                }
            }

            AccountDetailsInput::CapeUpdated(res) => {
                self.updating_cape = false;
                match res {
                    Ok(_) => {
                        self.cape_status = "Cape updated successfully".to_string();
                        for (idx, cape) in self.capes.iter_mut().enumerate() {
                            if (idx + 1) as u32 == self.selected_cape_idx {
                                cape.state = "ACTIVE".to_string();
                            } else {
                                cape.state = "INACTIVE".to_string();
                            }
                        }
                    }
                    Err(e) => {
                        self.cape_status = format!("Failed: {}", e);
                    }
                }
            }
        }
    }
}

impl AccountDetailsDialog {
    fn get_account_type_str(&self) -> String {
        match self.account.as_ref().map(|a| &a.account_type) {
            Some(AccountType::Microsoft) => "Microsoft Account".to_string(),
            Some(AccountType::Offline) => "Offline Account".to_string(),
            None => "Unknown".to_string(),
        }
    }

    fn get_status_str(&self) -> String {
        if let Some(ref acct) = self.account {
            let status = verify_account_status(acct);
            match status {
                AccountStatus::Valid => "Ready / Valid".to_string(),
                AccountStatus::ExpiringSoon => "Expiring Soon".to_string(),
                AccountStatus::Expired => "Expired (Refresh required)".to_string(),
                AccountStatus::Unknown(e) => format!("Unknown: {}", e),
            }
        } else {
            "N/A".to_string()
        }
    }

    fn get_expiry_str(&self) -> String {
        if let Some(ref acct) = self.account {
            if acct.account_type == AccountType::Offline || acct.token_expiry == 0 {
                return "Never".to_string();
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if acct.token_expiry > now {
                let diff = acct.token_expiry - now;
                let mins = diff / 60;
                let hours = mins / 60;
                if hours > 0 {
                    format!("Expires in ~{} hours", hours)
                } else {
                    format!("Expires in ~{} minutes", mins)
                }
            } else {
                "Expired".to_string()
            }
        } else {
            "N/A".to_string()
        }
    }
}
