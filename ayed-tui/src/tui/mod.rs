use std::{
    io::{stdin, stdout, Stdout, Write},
    sync::mpsc::Receiver,
};

use ayed_core::ui_state::{Color, Span};
use termion::{
    cursor::HideCursor,
    event::{Event, Key},
    input::{MouseTerminal, TermRead},
    raw::{IntoRawMode, RawTerminal},
};

pub mod panel;
pub mod renderer;

pub struct Tui {
    core: ayed_core::core::Core,
    screen: HideCursor<MouseTerminal<RawTerminal<Stdout>>>,
}

impl Tui {
    pub fn new(core: ayed_core::core::Core) -> Self {
        let stdout = stdout().into_raw_mode().unwrap();
        Self {
            core,
            screen: HideCursor::from(MouseTerminal::from(stdout)),
        }
    }

    pub fn run(&mut self) {
        let event_channel = Self::spawn_event_channel();

        while !self.core.is_quit() {
            self.render();

            let event = match event_channel.try_recv() {
                Ok(event) => event,
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(32));
                    continue;
                }
                e => e.unwrap(),
            };
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
        }

        self.cleanup_styling();
    }

    fn spawn_event_channel() -> Receiver<Event> {
        let (tx, rx) = std::sync::mpsc::channel::<Event>();
        std::thread::spawn(move || {
            for result_event in stdin().events() {
                let event = result_event.unwrap();
                tx.send(event).unwrap();
            }
        });
        rx
    }

    fn render(&mut self) {
        fn prepare_span_style(span: &Span, screen: &mut impl Write) {
            if let Some(foreground_color) = span.style.foreground_color {
                let fg = convert_color(foreground_color);
                write!(screen, "{}", termion::color::Fg(fg)).unwrap();
            }
            if let Some(background_color) = span.style.background_color {
                let bg = convert_color(background_color);
                write!(screen, "{}", termion::color::Bg(bg)).unwrap();
            }
            if span.style.invert {
                write!(screen, "{}", termion::style::Invert).unwrap();
            }
        }

        fn cleanup_span_style(screen: &mut impl Write) {
            write!(
                screen,
                "{}{}{}",
                termion::color::Reset.fg_str(),
                termion::color::Reset.bg_str(),
                termion::style::Reset
            )
            .unwrap();
        }

        self.update_viewport_size_if_needed();

        let ui_state = self.core.ui_state();

        //write!(self.screen, "{}", termion::clear::All).unwrap(); // This makes the display blink sometimes

        for panel in ui_state.panels {
            let start_y = panel.position.1;
            let after_end_y = start_y + panel.size.1;
            let start_x = panel.position.0;
            let after_end_x = start_x + panel.size.0;

            for (y, line) in (start_y..after_end_y).zip(panel.content.iter()) {
                write!(
                    self.screen,
                    "{}",
                    termion::cursor::Goto((start_x + 1) as _, (y + 1) as _)
                )
                .unwrap();

                cleanup_span_style(&mut self.screen);

                let panel_line_index = y - panel.position.1;
                let mut char_str = String::new();
                for (x, ch) in (start_x..after_end_x).zip(line.chars()) {
                    if panel
                        .spans_on_line(panel_line_index)
                        .filter(|span| span.to.column_index == x)
                        .next()
                        .is_some()
                    {
                        cleanup_span_style(&mut self.screen);
                    }
                    if let Some(span) = panel
                        .spans_on_line(panel_line_index)
                        .filter(|span| span.from.column_index == x)
                        .next()
                    {
                        prepare_span_style(span, &mut self.screen);
                    }

                    char_str.clear();
                    char_str.push(ch);
                    self.screen.write(char_str.as_bytes()).unwrap();
                }
            }
        }

        self.screen.flush().unwrap();
    }

    fn cleanup_styling(&mut self) {
        write!(
            self.screen,
            "{}{}{}",
            termion::color::Reset.fg_str(),
            termion::color::Reset.bg_str(),
            termion::style::Reset
        )
        .unwrap();
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

fn convert_color(color: Color) -> termion::color::Rgb {
    termion::color::Rgb(color.r, color.g, color.b)
}
