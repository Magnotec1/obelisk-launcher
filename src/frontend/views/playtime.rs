use crate::backend::playtime::PlaytimeManager;
use crate::frontend::app::AppMsg;
use adw::prelude::*;
use chrono::{DateTime, Utc};
use relm4::gtk;
use relm4::prelude::*;

#[derive(Debug)]
pub enum PlaytimeInput {
    UpdateData(PlaytimeManager, Vec<(String, String, u64, bool)>), // manager, (id, name, seconds, is_detected)
    Refresh,
    ResetRefreshing,
}

#[derive(Debug)]
pub enum PlaytimeOutput {
    Refresh,
}

pub struct PlaytimeView {
    loading: bool,
    total_seconds: u64,
    instance_data: Vec<(String, String, u64, Option<DateTime<Utc>>, usize, bool)>, // id, name, seconds, last_played, session_count, is_detected
    recent_sessions: Vec<(String, u64, DateTime<Utc>)>,              // name, duration, end_time
    list_box: gtk::ListBox,
    history_list_box: gtk::ListBox,
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

fn show_playtime_details_dialog(
    parent: &gtk::Window,
    name: &str,
    total_seconds: u64,
    last_played: Option<DateTime<Utc>>,
    session_count: usize,
    is_detected: bool,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(name)
        .close_response("close")
        .build();
    dialog.add_response("close", "Close");

    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .margin_start(16)
        .margin_end(16)
        .margin_bottom(16)
        .build();

    let details_list = gtk::ListBox::new();
    details_list.set_css_classes(&["boxed-list"]);
    details_list.set_selection_mode(gtk::SelectionMode::None);

    // Row 1: Status
    let status_row = adw::ActionRow::new();
    status_row.set_title("Status");
    let status_lbl = if is_detected {
        let label = gtk::Label::new(Some("Detected"));
        label.add_css_class("success");
        label
    } else {
        let label = gtk::Label::new(Some("Not Detected"));
        label.add_css_class("dim-label");
        label
    };
    status_row.add_suffix(&status_lbl);
    details_list.append(&status_row);

    // Row 2: Total Playtime
    let time_row = adw::ActionRow::new();
    time_row.set_title("Total Playtime");
    let time_lbl = gtk::Label::new(Some(&format_duration(total_seconds)));
    time_lbl.add_css_class("numeric");
    time_row.add_suffix(&time_lbl);
    details_list.append(&time_row);

    // Row 3: Last Played
    if let Some(lp) = last_played {
        let lp_row = adw::ActionRow::new();
        lp_row.set_title("Last Played");
        // Localized/nice formatting for the date
        let formatted_date = lp.with_timezone(&chrono::Local)
            .format("%B %e, %Y at %l:%M %p")
            .to_string();
        let lp_lbl = gtk::Label::new(Some(&formatted_date));
        lp_row.add_suffix(&lp_lbl);
        details_list.append(&lp_row);
    }

    // Row 4: Total Sessions
    let sessions_row = adw::ActionRow::new();
    sessions_row.set_title("Total Sessions");
    let sessions_lbl = gtk::Label::new(Some(&session_count.to_string()));
    sessions_row.add_suffix(&sessions_lbl);
    details_list.append(&sessions_row);

    container.append(&details_list);

    let clamp = adw::Clamp::builder()
        .maximum_size(450)
        .child(&container)
        .build();

    dialog.set_extra_child(Some(&clamp));
    dialog.choose(parent, None::<&gtk::gio::Cancellable>, |_| {});
}

impl PlaytimeView {
    fn rebuild_lists(&mut self) {
        // 1. Rebuild Instance List
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for (_id, name, seconds, last_played, session_count, is_detected) in &self.instance_data {
            let row = adw::ActionRow::new();
            if *is_detected {
                row.set_title(name);
            } else {
                row.set_title(&format!("{} (Not Detected)", name));
                row.add_css_class("dim-label");
            }

            let mut subtitle = format_duration(*seconds);
            if !*is_detected {
                subtitle.push_str(" • Playtime loaded from file");
            }
            row.set_subtitle(&subtitle);

            let percentage = if self.total_seconds > 0 {
                (*seconds as f64 / self.total_seconds as f64) * 100.0
            } else {
                0.0
            };

            let label = gtk::Label::new(Some(&format!("{:.1}%", percentage)));
            label.set_css_classes(&["dim-label", "numeric"]);
            row.add_suffix(&label);

            row.set_activatable(true);
            let name_clone = name.clone();
            let seconds_clone = *seconds;
            let last_played_clone = *last_played;
            let session_count_clone = *session_count;
            let is_detected_clone = *is_detected;

            let list_box_weak = self.list_box.downgrade();
            row.connect_activated(move |_| {
                if let Some(list_box) = list_box_weak.upgrade() {
                    if let Some(root) = list_box.root() {
                        if let Some(window) = root.downcast_ref::<gtk::Window>() {
                            show_playtime_details_dialog(
                                window,
                                &name_clone,
                                seconds_clone,
                                last_played_clone,
                                session_count_clone,
                                is_detected_clone,
                            );
                        }
                    }
                }
            });

            self.list_box.append(&row);
        }

        // 2. Rebuild History List
        while let Some(child) = self.history_list_box.first_child() {
            self.history_list_box.remove(&child);
        }

        if self.recent_sessions.is_empty() {
            let row = adw::ActionRow::new();
            row.set_title("No recent activity");
            self.history_list_box.append(&row);
        } else {
            for (name, duration, end_time) in &self.recent_sessions {
                let row = adw::ActionRow::new();
                row.set_title(name);
                row.set_subtitle(&format!(
                    "{} • {}",
                    format_duration(*duration),
                    end_time.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M")
                ));

                let icon = gtk::Image::from_icon_name("document-open-recent-symbolic");
                icon.set_css_classes(&["dim-label"]);
                row.add_prefix(&icon);

                self.history_list_box.append(&row);
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for PlaytimeView {
    type Init = ();
    type Input = PlaytimeInput;
    type Output = AppMsg;

    view! {
        adw::Bin {
            gtk::Stack {
                set_vexpand: true,

                add_named[Some("loading")] = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    set_spacing: 16,

                    adw::Spinner {
                        set_width_request: 64,
                        set_height_request: 64,
                    },

                    gtk::Label {
                        set_label: "Refreshing playtime data...",
                        set_css_classes: &["dim-label"],
                    }
                },

                add_named[Some("content")] = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vexpand: true,

                    adw::Clamp {
                        set_maximum_size: 1024,
                        set_tightening_threshold: 400,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 16,
                            set_spacing: 16,

                            // Summary / info card
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_css_classes: &["card"],

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_margin_all: 16,
                                    set_spacing: 8,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_hexpand: true,
                                        set_spacing: 2,

                                        gtk::Label {
                                            set_label: "Total Playtime",
                                            set_css_classes: &["heading"],
                                            set_halign: gtk::Align::Start,
                                        },
                                        gtk::Label {
                                            #[watch]
                                            set_label: &format_duration(model.total_seconds),
                                            set_css_classes: &["title-1"],
                                            set_halign: gtk::Align::Start,
                                        },
                                    },
                                    // Refresh button can go here if needed, but none for now
                                },
                            },

                            // History list
                            gtk::Label {
                                set_label: "Recent Activity",
                                set_css_classes: &["heading"],
                                set_halign: gtk::Align::Start,
                                set_margin_start: 4,
                                set_margin_top: 8,
                            },

                            #[local_ref]
                            history_list_box_ref -> gtk::ListBox {
                                set_css_classes: &["boxed-list"],
                                set_selection_mode: gtk::SelectionMode::None,
                            },

                            // Instance list
                            gtk::Label {
                                set_label: "All Instances",
                                set_css_classes: &["heading"],
                                set_halign: gtk::Align::Start,
                                set_margin_start: 4,
                                set_margin_top: 8,
                            },

                            #[local_ref]
                            list_box_ref -> gtk::ListBox {
                                set_css_classes: &["boxed-list"],
                                set_selection_mode: gtk::SelectionMode::None,
                            }
                        },
                    },
                },

                // Must come after add_named so children exist on first render
                #[watch]
                set_visible_child_name: if model.loading { "loading" } else { "content" },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let history_list_box = gtk::ListBox::new();
        history_list_box.set_css_classes(&["boxed-list"]);
        history_list_box.set_selection_mode(gtk::SelectionMode::None);

        let list_box = gtk::ListBox::new();
        list_box.set_css_classes(&["boxed-list"]);
        list_box.set_selection_mode(gtk::SelectionMode::None);

        let model = PlaytimeView {
            loading: false,
            total_seconds: 0,
            instance_data: Vec::new(),
            recent_sessions: Vec::new(),
            list_box: list_box.clone(),
            history_list_box: history_list_box.clone(),
        };

        let list_box_ref = &model.list_box;
        let history_list_box_ref = &model.history_list_box;

        let widgets = view_output!();

        sender.input(PlaytimeInput::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlaytimeInput::UpdateData(manager, data) => {
                use std::collections::HashMap;
                let name_map: HashMap<String, String> = data
                    .iter()
                    .map(|(id, name, _, _)| (id.clone(), name.clone()))
                    .collect();

                self.instance_data = data
                    .into_iter()
                    .map(|(id, name, seconds, is_detected)| {
                        let stored_last_played = manager
                            .instances
                            .get(&id)
                            .and_then(|inst_data| inst_data.last_played);

                        let session_count = manager
                            .instances
                            .get(&id)
                            .map(|inst_data| inst_data.session_count)
                            .unwrap_or(0);

                        let instance_sessions: Vec<_> = manager
                            .sessions
                            .iter()
                            .filter(|s| s.instance_id == id)
                            .cloned()
                            .collect();

                        let last_played = stored_last_played.or_else(|| {
                            instance_sessions.iter().map(|s| s.end_time).max()
                        });

                        (id, name, seconds, last_played, session_count, is_detected)
                    })
                    .collect();

                // Calculate recent sessions (last 5)
                let mut all_sessions = manager.sessions.clone();
                all_sessions.sort_by(|a, b| b.end_time.cmp(&a.end_time));

                self.recent_sessions = all_sessions
                    .into_iter()
                    .take(5)
                    .map(|s| {
                        let name = name_map
                            .get(&s.instance_id)
                            .cloned()
                            .unwrap_or_else(|| "Unknown Instance".to_string());
                        (name, s.duration_seconds, s.end_time)
                    })
                    .collect();

                self.total_seconds = self.instance_data.iter().map(|(_, _, s, _, _, _)| s).sum();
                self.instance_data.sort_by(|a, b| b.2.cmp(&a.2));

                self.rebuild_lists();

                let sender = sender.clone();
                relm4::spawn_local(async move {
                    gtk::glib::timeout_future(std::time::Duration::from_millis(500)).await;
                    sender.input(PlaytimeInput::ResetRefreshing);
                });
            }
            PlaytimeInput::Refresh => {
                self.loading = true;
                let _ = sender.output(AppMsg::RefreshPlaytime);
            }
            PlaytimeInput::ResetRefreshing => {
                self.loading = false;
            }
        }
    }
}
