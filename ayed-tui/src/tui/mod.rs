use std::io::{stdin, stdout, Write};

use termion::{
    cursor::HideCursor,
    event::{Event, Key},
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::AlternateScreen,
};

pub mod panel;
pub mod renderer;

pub struct Tui {
    core: ayed_core::core::Core,
}

impl Tui {
    pub fn new(core: ayed_core::core::Core) -> Self {
        Self { core }
    }

    pub fn run(&mut self) {
        let stdin = stdin();
        let stdout = stdout().into_raw_mode().unwrap();
        let mut screen = HideCursor::from(MouseTerminal::from(AlternateScreen::from(stdout)));

        self.render(&mut screen);

        // TODO non blocking event loop, one of these days
        for result_event in stdin.events() {
            let event = result_event.unwrap();
            match event {
                Event::Key(key) => match key {
                    Key::Esc => break,
                    Key::Backspace => self.core.input(ayed_core::input::Input::Backspace),
                    Key::Delete => self.core.input(ayed_core::input::Input::Delete),
                    Key::Up => self.core.input(ayed_core::input::Input::Up),
                    Key::Down => self.core.input(ayed_core::input::Input::Down),
                    Key::Left => self.core.input(ayed_core::input::Input::Left),
                    Key::Right => self.core.input(ayed_core::input::Input::Right),
                    Key::Char(ch) => self.core.input(ayed_core::input::Input::Char(ch)),
                    k => println!("key: {:?}", k),
                },
                e => {
                    println!("{:?}", e);
                }
            }

            self.render(&mut screen);
        }
    }

    fn render(&mut self, screen: &mut impl Write) {
        self.update_viewport_size_if_needed();

        let mut content = Vec::new();
        self.core.active_editor_viewport_content(&mut content);

        write!(
            screen,
            "{}{}",
            termion::clear::All,
            termion::cursor::Goto(1, 1)
        )
        .unwrap();

        for (i, line) in content.iter().enumerate() {
            screen.write_all(line.as_bytes()).unwrap();
            if i != content.len() - 1 {
                screen.write_all(&[b'\r', b'\n']).unwrap();
            }
        }

        screen.flush().unwrap();
    }

    fn update_viewport_size_if_needed(&mut self) {
        let (width, height) = self.viewport_size();
        let (vwidth, vheight) = self.core.viewport_size();
        if width != vwidth || height != vheight {
            self.core.set_viewport_size((width as _, height as _));
        }
    }

    fn viewport_size(&self) -> (u32, u32) {
        let (width, height) = termion::terminal_size().unwrap();
        (width as _, height as _)
    }
}
