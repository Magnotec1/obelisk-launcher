use adw::prelude::*;

/// Find the innermost / topmost visible `adw::ToastOverlay` currently on screen.
pub fn find_foremost_toast_overlay(start: &gtk::Widget) -> Option<adw::ToastOverlay> {
    let mut overlays = Vec::new();

    // 1. Check all visible top-level windows
    for toplevel in gtk::Window::list_toplevels() {
        if toplevel.is_visible() {
            collect_visible_toast_overlays(toplevel.upcast_ref(), &mut overlays);
        }
    }

    // 2. Traversal starting from the specified target widget if top-levels didn't find any
    if overlays.is_empty() {
        collect_visible_toast_overlays(start, &mut overlays);
    }

    overlays.pop() // Return the last (innermost / topmost) visible ToastOverlay
}

fn collect_visible_toast_overlays(widget: &gtk::Widget, overlays: &mut Vec<adw::ToastOverlay>) {
    if !widget.is_visible() {
        return;
    }
    if let Some(overlay) = widget.downcast_ref::<adw::ToastOverlay>() {
        overlays.push(overlay.clone());
    }
    let mut child = widget.first_child();
    while let Some(c) = child {
        collect_visible_toast_overlays(&c, overlays);
        child = c.next_sibling();
    }
}

/// Display a toast message on the foremost visible `adw::ToastOverlay`.
pub fn show_toast<W: IsA<gtk::Widget>>(start: &W, message: impl Into<String>) {
    let widget: &gtk::Widget = start.upcast_ref();
    if let Some(overlay) = find_foremost_toast_overlay(widget) {
        let toast = adw::Toast::new(&message.into());
        overlay.add_toast(toast);
    }
}
