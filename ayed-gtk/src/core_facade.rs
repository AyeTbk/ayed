use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;

use gtk::prelude::*;

use ayed_core::editor::Editor;
use ayed_core::input::Input;

pub struct CoreFacade {
    self_rc: RefCell<Option<Rc<Self>>>,
    core: RefCell<Editor>,
    gui_widget: RefCell<Option<gtk::ScrolledWindow>>,
    text_view_widget: RefCell<Option<gtk::TextView>>,
}

impl CoreFacade {
    pub fn new(core: Editor) -> Rc<Self> {
        let this = Self {
            self_rc: RefCell::new(None),
            core: RefCell::new(core),
            gui_widget: RefCell::new(None),
            text_view_widget: RefCell::new(None),
        };
        let self_rc = Rc::new(this);
        self_rc.self_rc.replace(Some(self_rc.clone()));
        self_rc
    }

    pub fn self_rc(&self) -> Rc<Self> {
        self.self_rc.borrow().as_ref().unwrap().clone()
    }

    pub fn init(&self) {
        let text_view = gtk::TextView::new();
        text_view.set_size_request(1, 1);
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_monospace(true);
        let text_buffer = gtk::TextBuffer::new(None);
        text_buffer.set_text("if you read this, something went wrong");
        text_view.set_buffer(Some(&text_buffer));

        let event_controller_key = gtk4::EventControllerKey::new();
        event_controller_key.connect_key_pressed({
            let facade = self.self_rc();
            move |_, key, _key_code, _modifiers| {
                facade.input_key(key);
                gtk4::Inhibit(true)
            }
        });
        text_view.add_controller(&event_controller_key);

        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_child(Some(&text_view));

        let this = self.self_rc();
        this.text_view_widget.replace(Some(text_view));
        this.gui_widget.replace(Some(scrolled_window));

        // TODO Setup a DrawingArea to be able to receive resize events
        // to be able to adjust the viewport size

        self.refresh_display();
    }

    pub fn gui_widget(&self) -> impl IsA<gtk4::Widget> {
        self.gui_widget.borrow().as_ref().unwrap().clone()
    }

    pub fn input_key(&self, key: gtk4::gdk::Key) {
        if let Some(ch) = key.to_unicode() {
            self.core.borrow_mut().input(Input::Char(ch));
            self.refresh_display();
            return;
        }
        use gtk4::gdk::Key;
        let input = match key {
            Key::Up => Input::Up,
            Key::Down => Input::Down,
            Key::Left => Input::Left,
            Key::Right => Input::Right,
            _ => return,
        };
        self.core.borrow_mut().input(input);
        self.refresh_display();
    }

    pub fn on_window_resize(&self) {
        dbg!("Wpppp");
    }

    pub fn refresh_display(&self) {
        let text_buffer = self.text_view_widget.borrow().as_ref().unwrap().buffer();
        let mut content = String::new();
        self.core
            .borrow()
            .active_buffer_viewport_content(&mut content);
        text_buffer.set_text(&content);

        self.refresh_content_tags();
    }

    fn refresh_content_tags(&self) {
        let text_buffer = self.text_view_widget.borrow().as_ref().unwrap().buffer();
        text_buffer.remove_all_tags(&text_buffer.start_iter(), &text_buffer.end_iter());

        let tag = gtk4::TextTag::new(None);
        tag.set_background_rgba(Some(&gtk4::gdk::RGBA::BLUE));

        text_buffer.tag_table().add(&tag);
        text_buffer.apply_tag(&tag, &text_buffer.start_iter(), &text_buffer.end_iter());
    }
}
