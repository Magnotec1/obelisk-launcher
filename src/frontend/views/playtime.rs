use crate::backend::playtime::PlaytimeManager;
use crate::frontend::app::AppMsg;
use adw::prelude::*;
use chrono::{DateTime, Utc};
use relm4::gtk;
use relm4::prelude::*;

#[derive(Debug)]
pub enum PlaytimeInput {
    UpdateData(PlaytimeManager, Vec<(String, String, u64)>), // manager, (id, name, seconds)
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
    instance_data: Vec<(String, u64, Option<DateTime<Utc>>, usize)>, // name, seconds, last_played, session_count
    recent_sessions: Vec<(String, u64, DateTime<Utc>)>,              // name, duration, end_time
    list_box: gtk::ListBox,
    history_list_box: gtk::ListBox,
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

impl PlaytimeView {
    fn rebuild_lists(&mut self) {
        // 1. Rebuild Instance List
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for (name, seconds, last_played, session_count) in &self.instance_data {
            let row = adw::ActionRow::new();
            row.set_title(name);

            let mut subtitle = format_duration(*seconds);
            if let Some(lp) = last_played {
                subtitle.push_str(&format!(" • Last played {}", lp.format("%Y-%m-%d")));
            }
            if *session_count > 0 {
                subtitle.push_str(&format!(" • {} sessions", session_count));
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
                    end_time.format("%Y-%m-%d %H:%M")
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
                    .map(|(id, name, _)| (id.clone(), name.clone()))
                    .collect();

                self.instance_data = data
                    .into_iter()
                    .map(|(id, name, seconds)| {
                        let instance_sessions: Vec<_> = manager
                            .sessions
                            .iter()
                            .filter(|s| s.instance_id == id)
                            .collect();

                        let last_played = instance_sessions.iter().map(|s| s.end_time).max();

                        let session_count = instance_sessions.len();

                        (name, seconds, last_played, session_count)
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

                self.total_seconds = self.instance_data.iter().map(|(_, s, _, _)| s).sum();
                self.instance_data.sort_by(|a, b| b.1.cmp(&a.1));

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
