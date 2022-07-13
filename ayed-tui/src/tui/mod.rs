use std::{
    io::{stdout, Stdout, Write},
    time::Duration,
};

use ayed_core::{
    input::Input,
    ui_state::{Color, Span},
};
use crossterm::{
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEvent},
    style::{SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

pub mod panel;
pub mod renderer;

pub struct Tui {
    core: ayed_core::core::Core,
    screen: Stdout,
}

impl Tui {
    pub fn new(core: ayed_core::core::Core) -> Self {
        let stdout = stdout();
        Self {
            core,
            screen: stdout,
        }
    }

    pub fn run(&mut self) {
        self.to_alternate_screen();

        self.render();

        while !self.core.is_quit() {
            if !crossterm::event::poll(Duration::from_millis(1000)).unwrap() {
                continue;
            }
            let event = crossterm::event::read().unwrap();

            match event {
                Event::Key(KeyEvent { code, .. }) => match code {
                    KeyCode::Esc => break,
                    KeyCode::Backspace => self.core.input(ayed_core::input::Key::Backspace.into()),
                    KeyCode::Delete => self.core.input(ayed_core::input::Key::Delete.into()),
                    KeyCode::Up => self.core.input(ayed_core::input::Key::Up.into()),
                    KeyCode::Down => self.core.input(ayed_core::input::Key::Down.into()),
                    KeyCode::Left => self.core.input(ayed_core::input::Key::Left.into()),
                    KeyCode::Right => self.core.input(ayed_core::input::Key::Right.into()),
                    KeyCode::Enter => self.core.input(Input::from_char('\n')),
                    KeyCode::Tab => self.core.input(Input::from_char('\t')),
                    KeyCode::Char(ch) => self.core.input(Input::from_char(ch)),
                    k => println!("key: {:?}", k),
                },
                Event::Resize(_, _) => (),
                e => {
                    println!("{:?}", e);
                }
            }

            self.render();
        }

        self.to_main_screen();
    }

    fn render(&mut self) {
        fn prepare_span_style(span: &Span, screen: &mut impl Write) {
            if let Some(foreground_color) = span.style.foreground_color {
                let fg = convert_color(foreground_color);
                screen.execute(SetForegroundColor(fg)).unwrap();
            }
            if let Some(background_color) = span.style.background_color {
                let bg = convert_color(background_color);
                screen.execute(SetBackgroundColor(bg)).unwrap();
            }
            if span.style.invert {
                write!(screen, "{}", crossterm::style::Attribute::Reverse).unwrap();
            }
        }

        fn cleanup_span_style(screen: &mut impl Write) {
            write!(
                screen,
                "{}{}",
                crossterm::style::ResetColor,
                crossterm::style::Attribute::Reset
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
                self.screen
                    .execute(MoveTo((start_x) as _, (y) as _))
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

    fn to_alternate_screen(&mut self) {
        enable_raw_mode().unwrap();
        let default_panic_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let _ = unset_crossterm_styling();
            default_panic_hook(panic_info);
        }));

        self.screen
            .execute(EnterAlternateScreen)
            .unwrap()
            .execute(crossterm::cursor::Hide)
            .unwrap();
    }

    fn to_main_screen(&mut self) {
        unset_crossterm_styling().unwrap();
    }

    fn update_viewport_size_if_needed(&mut self) {
        let (width, height) = self.viewport_size();
        let (vwidth, vheight) = self.core.viewport_size();
        if width != vwidth || height != vheight {
            self.core.set_viewport_size((width as _, height as _));
        }
    }

    fn viewport_size(&self) -> (u32, u32) {
        let (width, height) = crossterm::terminal::size().unwrap();
        ((width) as _, (height) as _)
    }
}

fn convert_color(color: Color) -> crossterm::style::Color {
    crossterm::style::Color::Rgb {
        r: color.r,
        g: color.g,
        b: color.b,
    }
}

fn unset_crossterm_styling() -> std::io::Result<()> {
    disable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(LeaveAlternateScreen)?;
    write!(
        stdout,
        "{}{}{}",
        crossterm::style::ResetColor,
        crossterm::style::Attribute::Reset,
        crossterm::cursor::Show,
    )
}
