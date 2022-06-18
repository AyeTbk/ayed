use std::io::{stdin, stdout, Write};

use termion::{
    color,
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
        let mut screen = MouseTerminal::from(AlternateScreen::from(stdout));

        self.render(&mut screen);

        // TODO non blocking event loop, one of these days
        for result_event in stdin.events() {
            let event = result_event.unwrap();
            match event {
                Event::Key(Key::Esc) => break,
                Event::Key(Key::Char(ch)) => self.core.input(ayed_core::input::Input::Char(ch)),
                e => {
                    println!("{:?}", e);
                }
            }

            self.render(&mut screen);
        }
    }

    fn render(&mut self, screen: &mut impl Write) {
        self.update_viewport_size_if_needed();
        screen.flush().unwrap();
    }

    fn update_viewport_size_if_needed(&mut self) {
        let (width, height) = termion::terminal_size().unwrap();
        let (vwidth, vheight) = self.core.viewport_size();
        if width as u32 != vwidth || height as u32 != vheight {
            self.core.set_viewport_size((width as _, height as _));
        }
    }
}
