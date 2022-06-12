use gtk4 as gtk;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

fn main() {
    let app = Application::builder().application_id("ay.ed").build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(640)
            .title("Ayed")
            .build();

        // guistate.init();

        let text_view = gtk::TextView::new();
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_monospace(true);
        let text_buffer = gtk::TextBuffer::new(None);
        text_buffer.set_text("this is monospace!");
        text_view.set_buffer(Some(&text_buffer));
        window.set_child(Some(&text_view));

        window.show();
    });

    app.run();
}
