use gtk4 as gtk;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow};

use ayed_core::editor::Editor;

mod core_facade;
use core_facade::CoreFacade;

fn main() {
    let core_facade = CoreFacade::new(Editor::new());

    let app = Application::builder()
        .application_id("ay.ed")
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_activate(move |app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(800)
            .default_height(640)
            .title("Ayed")
            .build();

        core_facade.init();

        for arg in /*std::env::args().skip(1)*/ Some("Cargo.toml") {
            app.open(&[gtk::gio::File::for_path(&arg)], "");
            core_facade.open_file(std::path::Path::new(&arg));
        }

        window.set_child(Some(&core_facade.gui_widget()));

        window.show();
    });

    app.run();
}
