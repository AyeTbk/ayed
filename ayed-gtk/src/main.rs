use gtk4 as gtk;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use ayed_core::editor::Editor;

mod core_facade;
use core_facade::CoreFacade;

fn main() {
    let core_facade = CoreFacade::new(Editor::new());

    let app = Application::builder().application_id("ay.ed").build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(640)
            .title("Ayed")
            .build();

        core_facade.init();

        window.set_child(Some(&core_facade.gui_widget()));

        window.show();
    });

    app.run();
}
