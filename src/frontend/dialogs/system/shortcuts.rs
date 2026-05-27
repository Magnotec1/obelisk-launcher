use adw::prelude::*;
use relm4::prelude::*;

pub struct ShortcutsDialog;

#[derive(Debug)]
pub enum ShortcutsInput {
    Open,
}

#[relm4::component(pub)]
impl SimpleComponent for ShortcutsDialog {
    type Init = ();
    type Input = ShortcutsInput;
    type Output = ();

    view! {
        adw::Dialog {
            set_title: "Keyboard Shortcuts",
            set_content_width: 450,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Keyboard Shortcuts",
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_propagate_natural_height: true,

                    adw::PreferencesPage {
                        adw::PreferencesGroup {
                            set_title: "Navigation",
                            adw::ActionRow {
                                set_title: "Settings",
                                set_subtitle: "Configure launcher preferences",
                                add_suffix = &gtk::Box {
                                    set_valign: gtk::Align::Center,
                                    set_css_classes: &["shortcut-badge"],
                                    gtk::Label {
                                        set_label: "Ctrl + ,",
                                        set_css_classes: &["dim-label"],
                                    }
                                }
                            },
                            adw::ActionRow {
                                set_title: "Keyboard Shortcuts",
                                set_subtitle: "Show this help dialog",
                                add_suffix = &gtk::Box {
                                    set_valign: gtk::Align::Center,
                                    set_css_classes: &["shortcut-badge"],
                                    gtk::Label {
                                        set_label: "Ctrl + ?",
                                        set_css_classes: &["dim-label"],
                                    }
                                }
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
        _sender: ComponentSender<Self>
    ) -> ComponentParts<Self> {
        let model = ShortcutsDialog;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ShortcutsInput::Open => {
                // The window is shown by the parent
            }
        }
    }
}
