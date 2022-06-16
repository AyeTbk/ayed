use std::cell::RefCell;
use std::rc::Rc;

use ayed_core::buffer::SelectionBounds;
use gtk4 as gtk;

use gtk::prelude::*;

use ayed_core::editor::Editor;
use ayed_core::input::Input;

pub struct CoreFacade {
    self_rc: RefCell<Option<Rc<Self>>>,
    core: RefCell<Editor>,
    gui_widget: RefCell<Option<gtk::Overlay>>,
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
        text_view.set_vexpand(true);
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

        // This drawing area only exists because it's the only way I've found to
        // track window resize events.
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_size_request(1, 1);
        drawing_area.connect_resize({
            let facade = self.self_rc();
            move |_, _, _| {
                facade.on_window_resize();
            }
        });

        let overlay_container = gtk::Overlay::new();
        overlay_container.set_child(Some(&drawing_area));
        overlay_container.add_overlay(&scrolled_window);

        let this = self.self_rc();
        this.text_view_widget.replace(Some(text_view));
        this.gui_widget.replace(Some(overlay_container));
    }

    pub fn gui_widget(&self) -> impl IsA<gtk4::Widget> {
        self.gui_widget.borrow().as_ref().unwrap().clone()
    }

    pub fn input_key(&self, key: gtk4::gdk::Key) {
        use gtk4::gdk::Key;
        let input = match key {
            Key::Return => Input::Return,
            Key::BackSpace => Input::Backspace,
            Key::Delete => Input::Delete,
            Key::Up => Input::Up,
            Key::Down => Input::Down,
            Key::Left => Input::Left,
            Key::Right => Input::Right,
            _ => {
                if let Some(ch) = key.to_unicode() {
                    Input::Char(ch)
                } else {
                    return;
                }
            }
        };
        self.core.borrow_mut().input(input);
        self.refresh_display();
    }

    pub fn on_window_resize(&self) {
        let text_view = self.text_view_widget.borrow().as_ref().unwrap().clone();
        let rect = text_view.visible_rect();

        // FIXME stop assuming character size
        let char_pixel_width = 8;
        let char_pixel_height = 18;

        // FIXME stop assuming starting window size
        // Workaround for Gtk not telling what size the fucking drawing area is, drawing area
        // which only exists because gtk wont tell me when stuff gets fucking resized
        let (rect_width, rect_height) = if rect.width() == 0 {
            (799, 639)
        } else {
            (rect.width(), rect.height())
        };

        let width = (rect_width / char_pixel_width) as _;
        let height = (rect_height / char_pixel_height - 1) as _;
        self.core.borrow_mut().set_viewport_size((width, height));
        self.refresh_display();
    }

    pub fn refresh_display(&self) {
        let text_buffer = self.text_view_widget.borrow().as_ref().unwrap().buffer();

        let core = self.core.borrow();
        let mut content = Vec::new();
        core.active_buffer_viewport_content(&mut content);
        content.push(" ");

        let mut string_content = String::new();
        for line in content {
            string_content.push_str(&line);
            string_content.push_str(" \n");
        }

        text_buffer.set_text(&string_content);

        self.refresh_content_tags();
    }

    fn refresh_content_tags(&self) {
        let text_buffer = self.text_view_widget.borrow().as_ref().unwrap().buffer();
        text_buffer.remove_all_tags(&text_buffer.start_iter(), &text_buffer.end_iter());

        let selection_tag = gtk4::TextTag::new(None);
        selection_tag.set_background_rgba(Some(&gtk4::gdk::RGBA::BLUE));
        text_buffer.tag_table().add(&selection_tag);

        for SelectionBounds { from, to } in self.core.borrow().active_buffer_selections() {
            let start_iter = if let Some(iter) =
                text_buffer.iter_at_line_offset(from.line_index as i32, from.column_index as i32)
            {
                iter
            } else {
                eprintln!("ayed-gtk: warn: bad selection bounds {from:?}");
                continue;
            };
            let end_iter = if let Some(iter) =
                text_buffer.iter_at_line_offset(to.line_index as i32, to.column_index as i32)
            {
                iter
            } else {
                eprintln!("ayed-gtk: warn: bad selection bounds {to:?}");
                continue;
            };
            text_buffer.apply_tag(&selection_tag, &start_iter, &end_iter);
        }
    }
}
