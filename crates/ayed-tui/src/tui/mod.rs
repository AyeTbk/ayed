use std::{
    io::{self, Stdout, Write, stdout},
    time::Duration,
};

use ayed_core::{
    core::Core,
    input::{self, Input},
    ui::{Color, Size, ui_state::StyledRegion},
};

use crossterm::{
    ExecutableCommand,
    cursor::MoveTo,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
    style::{SetBackgroundColor, SetForegroundColor},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

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
            if !crossterm::event::poll(Duration::from_millis(1000)).unwrap() {
                continue;
            }
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
        fn cleanup_span_style(screen: &mut impl Write) -> io::Result<()> {
            write!(
                screen,
                "{}{}",
                crossterm::style::ResetColor,
                crossterm::style::Attribute::Reset
            )
        }
        fn prepare_span_style(span: &StyledRegion, screen: &mut impl Write) -> io::Result<()> {
            cleanup_span_style(screen)?;
            if let Some(foreground_color) = span.style.foreground_color {
                let fg = convert_color_to_crossterm(foreground_color);
                screen.execute(SetForegroundColor(fg))?;
            }
            if let Some(background_color) = span.style.background_color {
                let bg = convert_color_to_crossterm(background_color);
                screen.execute(SetBackgroundColor(bg))?;
            }
            if span.style.invert {
                write!(screen, "{}", crossterm::style::Attribute::Reverse)?;
            }
            if span.style.underlined {
                write!(screen, "{}", crossterm::style::Attribute::Underlined)?;
            }
            Ok(())
        }

        self.update_viewport_size_if_needed();

        let ui_state = self.core.render();

        //write!(self.screen, "{}", termion::clear::All).unwrap(); // This makes the display blink sometimes

        for mut panel in ui_state.panels.into_iter() {
            panel.normalize_spans();
            panel.fixup_weird_chars();

            let start_y = panel.position.row;
            let after_end_y = start_y + panel.size.row as i32;
            let start_x = panel.position.column;
            let after_end_x = start_x + panel.size.column as i32;

            for (y, line) in (start_y..after_end_y).zip(panel.content.iter()) {
                self.screen.execute(MoveTo((start_x) as _, (y) as _))?;

                cleanup_span_style(&mut self.screen)?;

                let panel_row = y - panel.position.row; // NOTE this line makes the row local to panel position
                let mut char_str = String::new();
                let spans_on_line = panel.spans_on_line(panel_row).collect::<Vec<_>>();
                for (x, ch) in (start_x..after_end_x).zip(line.chars()) {
                    let panel_column = x - panel.position.column; // NOTE this line makes the column local to panel position
                    if let Some(span) = spans_on_line
                        .iter()
                        .filter(|span| span.from.column == panel_column)
                        .next()
                    {
                        prepare_span_style(span, &mut self.screen)?;
                    }

                    char_str.clear();
                    char_str.push(ch);
                    self.screen.write(char_str.as_bytes())?;

                    if spans_on_line
                        .iter()
                        .filter(|span| span.to.column == panel_column)
                        .next()
                        .is_some()
                    {
                        cleanup_span_style(&mut self.screen)?;
                    }
                }
            }
        }

        self.render_error_message()?;

        self.screen.flush()?;
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
        ((width) as _, (height) as _).into()
    }
}

fn convert_color_to_crossterm(color: Color) -> crossterm::style::Color {
    crossterm::style::Color::Rgb {
        r: color.r,
        g: color.g,
        b: color.b,
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
    )
}
