use adw::prelude::*;
use relm4::gtk;

pub fn configure_and_show_popover(
    popover: &gtk::Popover,
    source_widget: &impl IsA<gtk::Widget>,
    x: f64,
    y: f64,
) {
    let (local_x, local_y) = if let Some(parent) = popover.parent() {
        if let Some(pt) = source_widget.compute_point(
            &parent,
            &gtk::graphene::Point::new(x as f32, y as f32),
        ) {
            (pt.x() as f64, pt.y() as f64)
        } else {
            (x, y)
        }
    } else {
        (x, y)
    };

    let (root_w, root_h, root_x, root_y) = if let Some(root) = source_widget.root() {
        let rw = root.width() as f64;
        let rh = root.height() as f64;
        if let Some(pt) = source_widget.compute_point(
            &root,
            &gtk::graphene::Point::new(x as f32, y as f32),
        ) {
            (rw, rh, pt.x() as f64, pt.y() as f64)
        } else {
            (rw, rh, x, y)
        }
    } else {
        (800.0, 600.0, x, y)
    };

    let (rect_y, position) = if root_y > root_h - 150.0 {
        ((local_y - 4.0) as i32, gtk::PositionType::Top)
    } else {
        ((local_y + 4.0) as i32, gtk::PositionType::Bottom)
    };

    let (rect_x, halign) = if root_x > root_w - 150.0 {
        ((local_x - 4.0) as i32, gtk::Align::End)
    } else {
        ((local_x + 4.0) as i32, gtk::Align::Start)
    };

    popover.set_has_arrow(false);
    popover.set_position(position);
    popover.set_halign(halign);
    popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
        rect_x,
        rect_y,
        1,
        1,
    )));

    popover.popup();
}
