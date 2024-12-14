#[derive(Debug, Default, Clone, Copy)]
pub struct Style {
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
    pub invert: bool,
    pub underlined: bool,
}

impl Style {
    pub fn with_foreground_color(&self, color: Color) -> Self {
        let mut this = *self;
        this.foreground_color = Some(color);
        this
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Self { r, g, b }
    }

    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const DARK_RED: Self = Self { r: 128, g: 0, b: 0 };

    pub fn from_hex(hex: &str) -> Result<Color, ()> {
        // Valid hexcodes are made exclusively of ascii characters, so working on bytes is ok.
        let hex = hex.as_bytes();
        if hex.len() != 7 {
            return Err(());
        }
        if hex[0] != b'#' {
            return Err(());
        }
        fn hex_digit_value(digit: u8) -> Option<u8> {
            match digit {
                b'a'..=b'f' => Some(digit - b'a' + 10),
                b'A'..=b'F' => Some(digit - b'A' + 10),
                b'0'..=b'9' => Some(digit - b'0'),
                _ => None,
            }
        }
        fn hex_value(first_char: u8, second_char: u8) -> Option<u8> {
            Some((hex_digit_value(first_char)? << 4) | hex_digit_value(second_char)?)
        }

        let r = hex_value(hex[1], hex[2]).ok_or(())?;
        let g = hex_value(hex[3], hex[4]).ok_or(())?;
        let b = hex_value(hex[5], hex[6]).ok_or(())?;

        Ok(Color::rgb(r, g, b))
    }
}

pub const DEFAULT_PRIORITY: u8 = 10;

pub fn priority_from_str(src: &str) -> Result<u8, ()> {
    if !src.starts_with("priority:") {
        return Err(());
    }
    let num_str = src.trim_start_matches("priority:");
    num_str.parse::<u8>().map_err(|_| ())
}
