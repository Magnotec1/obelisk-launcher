use gtk::gdk;
use gtk::prelude::*;
fn main() {
    let display = gdk::Display::default().unwrap();
    let seat = display.default_seat().unwrap();
    let pointer = seat.pointer().unwrap();
    let _ = pointer.modifier_state().contains(gdk::ModifierType::SHIFT_MASK);
}
