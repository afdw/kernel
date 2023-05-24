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
