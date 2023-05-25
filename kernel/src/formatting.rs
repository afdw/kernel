use embedded_graphics::pixelcolor::Rgb888;

#[derive(Clone, Copy, Debug)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Color {
    pub fn from_ansi_foreground(byte: u8) -> Option<Self> {
        match byte {
            30 => Some(Color::Black),
            31 => Some(Color::Red),
            32 => Some(Color::Green),
            33 => Some(Color::Yellow),
            34 => Some(Color::Blue),
            35 => Some(Color::Magenta),
            36 => Some(Color::Cyan),
            37 => Some(Color::White),
            90 => Some(Color::BrightBlack),
            91 => Some(Color::BrightRed),
            92 => Some(Color::BrightGreen),
            93 => Some(Color::BrightYellow),
            94 => Some(Color::BrightBlue),
            95 => Some(Color::BrightMagenta),
            96 => Some(Color::BrightCyan),
            97 => Some(Color::BrightWhite),
            _ => None,
        }
    }

    pub fn from_ansi_background(byte: u8) -> Option<Self> {
        match byte {
            40 => Some(Color::Black),
            41 => Some(Color::Red),
            42 => Some(Color::Green),
            43 => Some(Color::Yellow),
            44 => Some(Color::Blue),
            45 => Some(Color::Magenta),
            46 => Some(Color::Cyan),
            47 => Some(Color::White),
            100 => Some(Color::BrightBlack),
            101 => Some(Color::BrightRed),
            102 => Some(Color::BrightGreen),
            103 => Some(Color::BrightYellow),
            104 => Some(Color::BrightBlue),
            105 => Some(Color::BrightMagenta),
            106 => Some(Color::BrightCyan),
            107 => Some(Color::BrightWhite),
            _ => None,
        }
    }

    pub fn ansi_foreground(self) -> u8 {
        match self {
            Color::Black => 30,
            Color::Red => 31,
            Color::Green => 32,
            Color::Yellow => 33,
            Color::Blue => 34,
            Color::Magenta => 35,
            Color::Cyan => 36,
            Color::White => 37,
            Color::BrightBlack => 90,
            Color::BrightRed => 91,
            Color::BrightGreen => 92,
            Color::BrightYellow => 93,
            Color::BrightBlue => 94,
            Color::BrightMagenta => 95,
            Color::BrightCyan => 96,
            Color::BrightWhite => 97,
        }
    }

    pub fn ansi_background(self) -> u8 {
        match self {
            Color::Black => 40,
            Color::Red => 41,
            Color::Green => 42,
            Color::Yellow => 43,
            Color::Blue => 44,
            Color::Magenta => 45,
            Color::Cyan => 46,
            Color::White => 47,
            Color::BrightBlack => 100,
            Color::BrightRed => 101,
            Color::BrightGreen => 102,
            Color::BrightYellow => 103,
            Color::BrightBlue => 104,
            Color::BrightMagenta => 105,
            Color::BrightCyan => 106,
            Color::BrightWhite => 107,
        }
    }

    pub fn rgb_888(self) -> Rgb888 {
        fn rgb_888_from_code(raw_color: u32) -> Rgb888 {
            Rgb888::new((raw_color >> 16) as u8, (raw_color >> 8) as u8, raw_color as u8)
        }
        match self {
            Color::Black => rgb_888_from_code(0x000000),
            Color::Red => rgb_888_from_code(0xcd0000),
            Color::Green => rgb_888_from_code(0x00cd00),
            Color::Yellow => rgb_888_from_code(0xcdcd00),
            Color::Blue => rgb_888_from_code(0x0000cd),
            Color::Magenta => rgb_888_from_code(0xcd00cd),
            Color::Cyan => rgb_888_from_code(0x00cdcd),
            Color::White => rgb_888_from_code(0xe5e5e5),
            Color::BrightBlack => rgb_888_from_code(0x7f7f7f),
            Color::BrightRed => rgb_888_from_code(0xff0000),
            Color::BrightGreen => rgb_888_from_code(0x00ff00),
            Color::BrightYellow => rgb_888_from_code(0xffff00),
            Color::BrightBlue => rgb_888_from_code(0x5c5cff),
            Color::BrightMagenta => rgb_888_from_code(0xff00ff),
            Color::BrightCyan => rgb_888_from_code(0x00ffff),
            Color::BrightWhite => rgb_888_from_code(0xffffff),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub reset: bool,
    pub foreground_color: Option<Color>,
    pub background_color: Option<Color>,
}

impl Style {
    pub const RESET: Style = Style {
        reset: true,
        foreground_color: None,
        background_color: None,
    };

    pub fn parse(style_str: &str) -> Option<Self> {
        let style_str = style_str.strip_prefix("\u{1b}[")?;
        let style_str = style_str.strip_suffix('m')?;
        let mut style = Style {
            reset: false,
            foreground_color: None,
            background_color: None,
        };
        for command_str in style_str.split(';') {
            let command = command_str.parse().ok()?;
            if command == 0 {
                style.reset = true;
            }
            if let Some(foreground_color) = Color::from_ansi_foreground(command) {
                style.foreground_color = Some(foreground_color);
            }
            if let Some(background_color) = Color::from_ansi_background(command) {
                style.background_color = Some(background_color);
            }
        }
        Some(style)
    }

    pub fn merge(self, other: Style, default: Style) -> Style {
        if other.reset {
            default
        } else {
            Style {
                reset: false,
                foreground_color: other.foreground_color.or(self.foreground_color),
                background_color: other.background_color.or(self.background_color),
            }
        }
    }
}

impl core::fmt::Display for Style {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.reset {
            write!(f, "\u{1b}[0m")
        } else {
            match (self.foreground_color, self.background_color) {
                (None, None) => write!(f, ""),
                (Some(foreground_color), None) => write!(f, "\u{1b}[{}m", foreground_color.ansi_foreground()),
                (None, Some(background_color)) => write!(f, "\u{1b}[{}m", background_color.ansi_background()),
                (Some(foreground_color), Some(background_color)) => {
                    write!(f, "\u{1b}[{};{}m", foreground_color.ansi_foreground(), background_color.ansi_background())
                }
            }
        }
    }
}
