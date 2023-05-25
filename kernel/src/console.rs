use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use embedded_graphics::{
    pixelcolor::{Rgb888, RgbColor},
    Drawable,
};

const FONT: embedded_graphics::mono_font::MonoFont<'_> = embedded_graphics::mono_font::ascii::FONT_10X20;
const DEFAULT_STYLE: super::formatting::Style = super::formatting::Style {
    reset: false,
    foreground_color: Some(super::formatting::Color::White),
    background_color: Some(super::formatting::Color::Black),
};
const DEFAULT_CHARACTER: char = ' ';

struct BufferFrameBufferBackend<'a>(&'a mut [u32]);

impl<'a> embedded_graphics_framebuf::backends::FrameBufferBackend for BufferFrameBufferBackend<'a> {
    type Color = Rgb888;

    fn set(&mut self, index: usize, color: Self::Color) {
        self.0[index] = (color.r() as u32) << 16 | (color.g() as u32) << 8 | (color.b() as u32);
    }

    fn get(&self, index: usize) -> Self::Color {
        let raw_color = self.0[index];
        Rgb888::new((raw_color >> 16) as u8, (raw_color >> 8) as u8, raw_color as u8)
    }

    fn nr_elements(&self) -> usize {
        self.0.len()
    }
}

pub struct Console {
    current_character_bytes: Vec<u8>,
    current_style_characters: Vec<char>,
    history: Vec<char>,
    width: usize,
    height: usize,
    styled_characters: Vec<Vec<(super::formatting::Style, char)>>,
    horizontal_advancement: usize,
    current_style: super::formatting::Style,
}

impl Console {
    pub fn new() -> Self {
        Console {
            current_character_bytes: vec![],
            current_style_characters: vec![],
            history: vec![],
            width: 0,
            height: 0,
            styled_characters: vec![],
            horizontal_advancement: 0,
            current_style: DEFAULT_STYLE,
        }
    }

    pub fn receive_byte(&mut self, byte: u8) {
        self.current_character_bytes.push(byte);
        match String::from_utf8(self.current_character_bytes.clone()) {
            Ok(str) => {
                assert_eq!(str.len(), 1);
                self.current_character_bytes.clear();
                let character = str.chars().next().unwrap();
                self.history.push(character);
                self.process_character(character);
            }
            Err(_) => assert!(self.current_character_bytes.len() < 4),
        }
    }

    fn process_character(&mut self, character: char) {
        if self.height == 0 {
            return;
        }
        if character == '\u{1b}' || !self.current_style_characters.is_empty() {
            self.current_style_characters.push(character);
            if character == 'm' {
                let style_str = core::mem::take(&mut self.current_style_characters).into_iter().collect::<String>();
                let style = super::formatting::Style::parse(&style_str);
                if let Some(style) = style {
                    self.current_style = self.current_style.merge(style, DEFAULT_STYLE);
                }
            }
            return;
        }
        if self.horizontal_advancement == self.width || character == '\n' {
            self.styled_characters.remove(0);
            self.styled_characters.push(vec![(DEFAULT_STYLE, DEFAULT_CHARACTER); self.width]);
            self.horizontal_advancement = 0;
            return;
        }
        self.styled_characters.last_mut().unwrap()[self.horizontal_advancement] = (self.current_style, character);
        self.horizontal_advancement += 1;
    }

    pub fn render<D: super::display::Display>(&mut self, display: &D) {
        let resolution = display.resolution();
        let mut pixel_data = vec![0; resolution.0 * resolution.1];
        let mut frame_buf = embedded_graphics_framebuf::FrameBuf::new(BufferFrameBufferBackend(&mut pixel_data), resolution.0, resolution.1);
        let width = resolution.0 / (FONT.character_size.width as usize);
        let height = resolution.1 / (FONT.character_size.height as usize);
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.styled_characters = vec![vec![(DEFAULT_STYLE, DEFAULT_CHARACTER); self.width]; self.height];
            self.horizontal_advancement = width;
            self.current_style = DEFAULT_STYLE;
            for character in self.history.clone() {
                self.process_character(character);
            }
        }
        for (y, styled_characters_line) in self.styled_characters.iter().enumerate() {
            for (x, &(style, character)) in styled_characters_line.iter().enumerate() {
                embedded_graphics::text::Text::with_text_style(
                    &character.to_string(),
                    embedded_graphics::prelude::Point::new(x as i32 * FONT.character_size.width as i32, y as i32 * FONT.character_size.height as i32),
                    embedded_graphics::mono_font::MonoTextStyleBuilder::new()
                        .font(&FONT)
                        .background_color(style.background_color.unwrap().rgb_888())
                        .text_color(style.foreground_color.unwrap().rgb_888())
                        .build(),
                    embedded_graphics::text::TextStyle::with_baseline(embedded_graphics::text::Baseline::Top),
                )
                .draw(&mut frame_buf)
                .unwrap();
            }
        }
        display.update(&pixel_data);
    }
}

impl acid_io::Write for Console {
    fn write(&mut self, src: &[u8]) -> acid_io::Result<usize> {
        for &byte in src {
            self.receive_byte(byte);
        }
        Ok(src.len())
    }

    fn flush(&mut self) -> acid_io::Result<()> {
        Ok(())
    }
}

impl core::fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        use acid_io::Write;
        self.write(s.as_bytes()).unwrap();
        Ok(())
    }
}
