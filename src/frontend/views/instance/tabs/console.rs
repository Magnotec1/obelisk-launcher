use adw::prelude::*;
use relm4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    All,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn from_line(line: &str) -> Self {
        let line_upper = line.to_uppercase();
        if line_upper.contains("/ERROR")
            || line_upper.contains(" ERROR ")
            || line_upper.contains("[ERROR]")
        {
            LogLevel::Error
        } else if line_upper.contains("/WARN")
            || line_upper.contains(" WARN ")
            || line_upper.contains("[WARN]")
        {
            LogLevel::Warn
        } else if line_upper.contains("/INFO")
            || line_upper.contains(" INFO ")
            || line_upper.contains("[INFO]")
        {
            LogLevel::Info
        } else {
            LogLevel::Info // Default to Info for simplicity
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub level: LogLevel,
    pub content: String,
}

pub struct InstanceConsole {
    pub buffer: gtk::TextBuffer,
    pub is_running: bool,
}

#[derive(Debug)]
pub enum ConsoleInput {
    Update(gtk::TextBuffer, bool),
}

#[derive(Debug)]
pub enum ConsoleOutput {
    Launch,
    Kill,
    Clear,
    SetFilter(LogLevel),
}

#[relm4::component(pub)]
#[allow(unused_assignments)]
impl SimpleComponent for InstanceConsole {
    type Init = (gtk::TextBuffer, bool);
    type Input = ConsoleInput;
    type Output = ConsoleOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,

            // Toolbar row: filter toggle group on the left, clear button on the right
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_top: 12,
                set_margin_bottom: 8,
                set_margin_start: 16,
                set_margin_end: 16,
                set_spacing: 8,

                #[name = "console_filter_box"]
                gtk::Box {
                    set_hexpand: true,
                },

                // Clear button on the far right
                gtk::Button {
                    set_icon_name: "edit-clear-all-symbolic",
                    set_tooltip_text: Some("Clear Console"),
                    set_css_classes: &["flat", "circular"],
                    connect_clicked[sender] => move |_| {
                        sender.output(ConsoleOutput::Clear).unwrap();
                    },
                },
            },

            #[name = "console_stack"]
            adw::ViewStack {
                set_vexpand: true,
                #[watch]
                set_visible_child_name: if model.buffer.char_count() == 0 { "empty" } else { "logs" },

                add_titled[Some("empty"), "Empty"] = &adw::StatusPage {
                    set_title: "No Logs Yet",
                    set_description: Some("Launch an instance to see the console output here."),
                    set_icon_name: Some("utilities-terminal-symbolic"),
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
                }
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
                    set_visible: !model.is_running,
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
                    set_visible: model.is_running,
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
            is_running: init.1,
        };

        let widgets = view_output!();

        widgets.console_stack.set_visible_child_name(if model.buffer.char_count() == 0 { "empty" } else { "logs" });

        // Set up the console filter ToggleGroup
        {
            let toggle_group = adw::ToggleGroup::new();

            let toggle_all = adw::Toggle::builder().name("all").label("All").build();
            let toggle_info = adw::Toggle::builder().name("info").label("Info").build();
            let toggle_warn = adw::Toggle::builder()
                .name("warn")
                .label("Warnings")
                .build();
            let toggle_error = adw::Toggle::builder().name("error").label("Errors").build();

            toggle_group.add(toggle_all);
            toggle_group.add(toggle_info);
            toggle_group.add(toggle_warn);
            toggle_group.add(toggle_error);

            toggle_group.set_active_name(Some("all"));

            let sender_clone = sender.clone();
            toggle_group.connect_active_name_notify(move |group| {
                if let Some(name) = group.active_name() {
                    let level = match name.as_str() {
                        "info" => LogLevel::Info,
                        "warn" => LogLevel::Warn,
                        "error" => LogLevel::Error,
                        _ => LogLevel::All,
                    };
                    sender_clone
                        .output(ConsoleOutput::SetFilter(level))
                        .unwrap();
                }
            });

            widgets.console_filter_box.append(&toggle_group);
        }

        // Auto-scroll console
        let adj = widgets.console_scrolled.vadjustment();
        adj.connect_changed(move |a: &gtk::Adjustment| {
            a.set_value(a.upper() - a.page_size());
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ConsoleInput::Update(buffer, running) => {
                self.buffer = buffer;
                self.is_running = running;
            }
        }
    }
}
