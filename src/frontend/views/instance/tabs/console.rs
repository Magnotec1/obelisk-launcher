use crate::frontend::app::InstanceStatus;
use adw::prelude::*;
use relm4::prelude::*;

pub struct InstanceConsole {
    pub buffer: gtk::TextBuffer,
    pub status: InstanceStatus,
    pub has_any_logs: bool,
}

#[derive(Debug)]
pub enum ConsoleInput {
    Update {
        buffer: gtk::TextBuffer,
        status: InstanceStatus,
        has_any_logs: bool,
    },
}

#[derive(Debug)]
pub enum ConsoleOutput {
    Launch,
    Kill,
    Search(String),
}

#[relm4::component(pub)]
#[allow(unused_assignments)]
impl SimpleComponent for InstanceConsole {
    type Init = (gtk::TextBuffer, InstanceStatus, bool);
    type Input = ConsoleInput;
    type Output = ConsoleOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,

            gtk::SearchEntry {
                set_placeholder_text: Some("Search logs..."),
                set_margin_top: 12,
                set_margin_bottom: 8,
                set_margin_start: 16,
                set_margin_end: 16,
                #[watch]
                set_visible: model.has_any_logs,
                connect_search_changed[sender] => move |entry| {
                    sender.output(ConsoleOutput::Search(entry.text().to_string())).unwrap();
                },
            },

            #[name = "console_stack"]
            adw::ViewStack {
                set_vexpand: true,

                add_titled[Some("empty"), "Empty"] = &adw::StatusPage {
                    set_title: "No Logs Yet",
                    set_description: Some("Launch an instance to see the console output here."),
                    set_icon_name: Some("utilities-terminal-symbolic"),
                },

                add_titled[Some("no_matches"), "No Matches"] = &adw::StatusPage {
                    set_title: "No Results Found",
                    set_description: Some("Try searching for something else."),
                    set_icon_name: Some("system-search-symbolic"),
                },

                #[name = "console_scrolled"]
                add_titled[Some("logs"), "Logs"] = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vexpand: true,
                    set_margin_start: 12,
                    set_margin_end: 12,
                    set_margin_bottom: 12,
                    set_css_classes: &["console-view"],

                    #[wrap(Some)]
                    set_child = &gtk::TextView {
                        set_editable: false,
                        set_cursor_visible: false,
                        set_left_margin: 12,
                        set_right_margin: 12,
                        set_top_margin: 12,
                        set_bottom_margin: 12,
                        set_monospace: true,
                        #[watch]
                        set_buffer: Some(&model.buffer),
                        set_css_classes: &["console-text"],
                    }
                },

                // Must come after add_titled so children exist on first render
                #[watch]
                set_visible_child_name: if !model.has_any_logs {
                    "empty"
                } else if model.buffer.char_count() == 0 {
                    "no_matches"
                } else {
                    "logs"
                },
            },

            // Bottom Action Row
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_halign: gtk::Align::Center,
                set_margin_top: 8,
                set_margin_bottom: 16,
                set_spacing: 12,

                gtk::Button {
                    #[watch]
                    set_visible: model.status == InstanceStatus::NotRunning,
                    set_css_classes: &["suggested-action", "pill"],
                    connect_clicked[sender] => move |_| {
                        sender.output(ConsoleOutput::Launch).unwrap();
                    },
                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Center,
                        gtk::Image { set_icon_name: Some("media-playback-start-symbolic") },
                        gtk::Label { set_label: "Launch Game" }
                    }
                },

                gtk::Button {
                    #[watch]
                    set_visible: model.status == InstanceStatus::Loading,
                    set_css_classes: &["pill"],
                    set_sensitive: false,
                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Center,
                        adw::Spinner {
                            set_width_request: 16,
                            set_height_request: 16,
                        },
                        gtk::Label { set_label: "Loading Game…" }
                    }
                },

                gtk::Button {
                    #[watch]
                    set_visible: model.status == InstanceStatus::Running,
                    set_css_classes: &["destructive-action", "pill"],
                    connect_clicked[sender] => move |_| {
                        sender.output(ConsoleOutput::Kill).unwrap();
                    },
                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Center,
                        gtk::Image { set_icon_name: Some("media-playback-stop-symbolic") },
                        gtk::Label { set_label: "Stop Game" }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InstanceConsole {
            buffer: init.0,
            status: init.1,
            has_any_logs: init.2,
        };

        let widgets = view_output!();

        widgets
            .console_stack
            .set_visible_child_name(if !model.has_any_logs {
                "empty"
            } else if model.buffer.char_count() == 0 {
                "no_matches"
            } else {
                "logs"
            });


        // Auto-scroll console
        let adj = widgets.console_scrolled.vadjustment();
        adj.connect_changed(move |a: &gtk::Adjustment| {
            a.set_value(a.upper() - a.page_size());
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ConsoleInput::Update {
                buffer,
                status,
                has_any_logs,
            } => {
                self.buffer = buffer;
                self.status = status;
                self.has_any_logs = has_any_logs;
            }
        }
    }
}
