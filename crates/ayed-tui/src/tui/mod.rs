use std::io::{self, Stdout, Write, stdout};

use ayed_core::{
    core::Core,
    input::{self, Input},
    ui::{Color, Size, Style},
};

use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    style::{Attribute, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    },
};

use crate::tui::render_buffer::RenderBufferCell;

mod render_buffer;

pub struct Tui {
    core: Core,
    screen: Stdout,
    error_message: Option<String>,
}

impl Tui {
    pub fn new(core: Core) -> Self {
        let stdout = stdout();
        Self {
            core,
            screen: stdout,
            error_message: None,
        }
    }

    pub fn run(&mut self) {
        self.to_alternate_screen();

        self.render().unwrap();

        while !self.core.quit_requested() {
            // if crossterm::event::poll(Duration::from_millis(50)).unwrap() {
            if true {
                let event = crossterm::event::read().unwrap();

                match event {
                    Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) => match convert_key_code_and_modifiers_to_ayed(code, modifiers) {
                        Ok((key, modifiers)) => {
                            let input = Input::new(key, modifiers);
                            self.core.emit_input_event(input);
                        }
                        Err(msg) => {
                            self.set_error_message(msg);
                        }
                    },
                    Event::Resize(_, _) => (),
                    e => {
                        self.set_error_message(format!("unhandled event: {:?}", e));
                    }
                }
            }

            self.core.tick();

            self.render().unwrap();
        }

        self.to_main_screen();
    }

    fn set_error_message(&mut self, mut msg: String) {
        msg.insert_str(0, "[tui error] ");
        self.error_message = Some(msg);
    }

    fn render(&mut self) -> io::Result<()> {
        self.update_viewport_size_if_needed();

        let ui_state = self.core.render();
        let rbuf = render_buffer::RenderBuffer::render(self.viewport_size(), ui_state);

        let mut screen = self.screen.lock();
        screen.execute(BeginSynchronizedUpdate)?;

        write!(screen, "{}{}", ResetColor, Attribute::Reset)?;

        for (y, line) in rbuf.buffer.into_iter().enumerate() {
            write!(screen, "{}", MoveTo(0 as _, y as _))?;

            let mut style = Style::default();
            for cell in line {
                Self::render_cell(&mut screen, &cell, &mut style)?
            }
        }

        self.render_error_message()?;

        screen.execute(EndSynchronizedUpdate)?;
        screen.flush()?;

        Ok(())
    }

    fn render_cell(
        screen: &mut impl Write,
        cell: &RenderBufferCell,
        style: &mut Style,
    ) -> io::Result<()> {
        // Prepare the style
        if style.foreground_color != cell.style.foreground_color {
            style.foreground_color = cell.style.foreground_color;
            let cmd = SetForegroundColor(convert_color_to_crossterm(style.foreground_color));
            write!(screen, "{}", cmd)?;
        }
        if style.background_color != cell.style.background_color {
            style.background_color = cell.style.background_color;
            let cmd = SetBackgroundColor(convert_color_to_crossterm(style.background_color));
            write!(screen, "{}", cmd)?;
        }
        if style.invert != cell.style.invert {
            style.invert = cell.style.invert;
            if style.invert {
                write!(screen, "{}", Attribute::Reverse)?;
            } else {
                write!(screen, "{}", Attribute::NoReverse)?;
            }
        }
        if style.bold != cell.style.bold {
            style.bold = cell.style.bold;
            if style.bold {
                write!(screen, "{}", Attribute::Bold)?;
            } else {
                write!(screen, "{}", Attribute::NormalIntensity)?;
            }
        }
        if style.underlined != cell.style.underlined {
            style.underlined = cell.style.underlined;
            if style.underlined {
                write!(screen, "{}", Attribute::Underlined)?;
            } else {
                write!(screen, "{}", Attribute::NoUnderline)?;
            }
        }
        // Write out char
        let mut char_buf = [0; 4];
        let bytes = cell.data.encode_utf8(&mut char_buf).as_bytes();
        screen.write(bytes)?;

        Ok(())
    }

    fn render_error_message(&mut self) -> io::Result<()> {
        let Some(msg) = self.error_message.take() else {
            return Ok(());
        };
        self.screen
            .execute(SetForegroundColor(convert_color_to_crossterm(Color::WHITE)))?;
        self.screen
            .execute(SetBackgroundColor(convert_color_to_crossterm(
                Color::DARK_RED,
            )))?;
        self.screen.execute(MoveTo(0, 0))?;
        self.screen.write(msg.as_bytes())?;
        Ok(())
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
        let size = self.viewport_size();
        let vsize = self.core.viewport_size();
        if size != vsize {
            self.core.set_viewport_size(size);
        }
    }

    fn viewport_size(&self) -> Size {
        let (width, height) = crossterm::terminal::size().unwrap();
        (width as i32, height as i32).into()
    }
}

fn convert_color_to_crossterm(color: impl Into<Option<Color>>) -> crossterm::style::Color {
    if let Some(color) = color.into() {
        crossterm::style::Color::Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    } else {
        crossterm::style::Color::Reset
    }
}

fn convert_key_code_and_modifiers_to_ayed(
    code: KeyCode,
    modifiers: KeyModifiers,
) -> Result<(input::Key, input::Modifiers), String> {
    let ayed_modifiers = convert_key_modifiers_to_ayed(modifiers);

    use input::Key as AyedKey;
    let ayed_code = match code {
        KeyCode::Backspace => AyedKey::Backspace,
        KeyCode::Delete => AyedKey::Delete,
        KeyCode::Home => AyedKey::Home,
        KeyCode::End => AyedKey::End,
        KeyCode::Up => AyedKey::Up,
        KeyCode::Down => AyedKey::Down,
        KeyCode::Left => AyedKey::Left,
        KeyCode::Right => AyedKey::Right,
        KeyCode::Enter => AyedKey::Char('\n'),
        KeyCode::Tab => AyedKey::Char('\t'),
        KeyCode::BackTab => AyedKey::BackTab,
        KeyCode::PageUp => AyedKey::PageUp,
        KeyCode::PageDown => AyedKey::PageDown,
        KeyCode::Char(ch) => AyedKey::Char(ch),
        KeyCode::Esc => AyedKey::Escape,
        k => {
            return Err(format!("key not implemented: {:?}", k));
        }
    };
    Ok((ayed_code, ayed_modifiers))
}

fn convert_key_modifiers_to_ayed(modifiers: KeyModifiers) -> input::Modifiers {
    let mut mods = input::Modifiers::default();
    if modifiers.contains(KeyModifiers::CONTROL) {
        mods = mods.with_ctrl();
    }
    if modifiers.contains(KeyModifiers::SHIFT) {
        mods = mods.with_shift();
    }
    if modifiers.contains(KeyModifiers::ALT) {
        mods = mods.with_alt();
    }
    mods
}

fn unset_crossterm_styling() -> io::Result<()> {
    disable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(LeaveAlternateScreen)?;
    write!(
        stdout,
        "{}{}{}",
        crossterm::style::ResetColor,
        crossterm::style::Attribute::Reset,
        crossterm::cursor::Show,
    )?;
    stdout.flush()
}
