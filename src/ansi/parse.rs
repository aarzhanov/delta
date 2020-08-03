use core::str::Bytes;

use ansi_term;
use vte;

#[derive(Debug, PartialEq)]
pub enum Element {
    Text(String),
    Style(ansi_term::Style),
}

pub struct AnsiCodeIterator<'a> {
    bytes: Bytes<'a>,
    performer: Performer,
}

struct Performer {
    style: Option<ansi_term::Style>,
    text: String,
}

impl<'a> AnsiCodeIterator<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            bytes: s.bytes(),
            performer: Performer {
                style: None,
                text: String::new(),
            },
        }
    }

    fn emit_text(&mut self) -> Option<String> {
        if !self.performer.text.is_empty() {
            let text = self.performer.text.clone();
            self.performer.text = String::new();
            Some(text)
        } else {
            None
        }
    }
}

impl<'a> Iterator for AnsiCodeIterator<'a> {
    type Item = (Element, bool);

    fn next(&mut self) -> Option<(Element, bool)> {
        if let Some(style) = self.performer.style {
            self.performer.style = None;
            return Some((Element::Style(style), true));
        }
        let mut machine = vte::Parser::new();
        loop {
            if let Some(byte) = self.bytes.next() {
                machine.advance(&mut self.performer, byte);
                if let Some(style) = self.performer.style {
                    if let Some(text) = self.emit_text() {
                        return Some((Element::Text(text), false));
                    }
                    self.performer.style = None;
                    return Some((Element::Style(style), true));
                }
            } else if let Some(text) = self.emit_text() {
                return Some((Element::Text(text), false));
            } else {
                return None;
            }
        }
    }
}

pub fn parse_first_style(s: &str) -> Option<ansi_term::Style> {
    let mut machine = vte::Parser::new();
    let mut performer = Performer {
        style: None,
        text: String::new(),
    };
    for b in s.bytes() {
        if performer.style.is_some() {
            return performer.style;
        }
        machine.advance(&mut performer, b)
    }
    None
}

// Based on https://github.com/alacritty/vte/blob/0310be12d3007e32be614c5df94653d29fcc1a8b/examples/parselog.rs
impl vte::Perform for Performer {
    fn csi_dispatch(&mut self, params: &[i64], intermediates: &[u8], ignore: bool, c: char) {
        if ignore || intermediates.len() > 1 {
            return;
        }

        match (c, intermediates.get(0)) {
            ('m', None) => {
                if params.is_empty() {
                    // Attr::Reset;
                } else {
                    self.style = Some(ansi_term_style_from_sgr_parameters(params))
                }
            }
            _ => {}
        }
    }

    fn print(&mut self, c: char) {
        self.text.push(c);
    }

    fn execute(&mut self, _byte: u8) {}

    fn hook(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool, _c: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

// Based on https://github.com/alacritty/alacritty/blob/57c4ac9145a20fb1ae9a21102503458d3da06c7b/alacritty_terminal/src/ansi.rs#L1168
fn ansi_term_style_from_sgr_parameters(parameters: &[i64]) -> ansi_term::Style {
    let mut i = 0;
    let mut style = ansi_term::Style::new();
    loop {
        if i >= parameters.len() {
            break;
        }

        match parameters[i] {
            // 0 => Some(Attr::Reset),
            1 => style.is_bold = true,
            2 => style.is_dimmed = true,
            3 => style.is_italic = true,
            4 => style.is_underline = true,
            5 => style.is_blink = true, // blink slow
            6 => style.is_blink = true, // blink fast
            7 => style.is_reverse = true,
            8 => style.is_hidden = true,
            9 => style.is_strikethrough = true,
            // 21 => Some(Attr::CancelBold),
            // 22 => Some(Attr::CancelBoldDim),
            // 23 => Some(Attr::CancelItalic),
            // 24 => Some(Attr::CancelUnderline),
            // 25 => Some(Attr::CancelBlink),
            // 27 => Some(Attr::CancelReverse),
            // 28 => Some(Attr::CancelHidden),
            // 29 => Some(Attr::CancelStrike),
            30 => style.foreground = Some(ansi_term::Color::Black),
            31 => style.foreground = Some(ansi_term::Color::Red),
            32 => style.foreground = Some(ansi_term::Color::Green),
            33 => style.foreground = Some(ansi_term::Color::Yellow),
            34 => style.foreground = Some(ansi_term::Color::Blue),
            35 => style.foreground = Some(ansi_term::Color::Purple),
            36 => style.foreground = Some(ansi_term::Color::Cyan),
            37 => style.foreground = Some(ansi_term::Color::White),
            38 => {
                let mut start = 0;
                if let Some(color) = parse_sgr_color(&parameters[i..], &mut start) {
                    i += start;
                    style.foreground = Some(color);
                }
            }
            // 39 => Some(Attr::Foreground(Color::Named(NamedColor::Foreground))),
            40 => style.background = Some(ansi_term::Color::Black),
            41 => style.background = Some(ansi_term::Color::Red),
            42 => style.background = Some(ansi_term::Color::Green),
            43 => style.background = Some(ansi_term::Color::Yellow),
            44 => style.background = Some(ansi_term::Color::Blue),
            45 => style.background = Some(ansi_term::Color::Purple),
            46 => style.background = Some(ansi_term::Color::Cyan),
            47 => style.background = Some(ansi_term::Color::White),
            48 => {
                let mut start = 0;
                if let Some(color) = parse_sgr_color(&parameters[i..], &mut start) {
                    i += start;
                    style.background = Some(color);
                }
            }
            // 49 => Some(Attr::Background(Color::Named(NamedColor::Background))),
            // "bright" colors. ansi_term doesn't offer a way to emit them as, e.g., 90m; instead
            // that would be 38;5;8.
            90 => style.foreground = Some(ansi_term::Color::Fixed(8)),
            91 => style.foreground = Some(ansi_term::Color::Fixed(9)),
            92 => style.foreground = Some(ansi_term::Color::Fixed(10)),
            93 => style.foreground = Some(ansi_term::Color::Fixed(11)),
            94 => style.foreground = Some(ansi_term::Color::Fixed(12)),
            95 => style.foreground = Some(ansi_term::Color::Fixed(13)),
            96 => style.foreground = Some(ansi_term::Color::Fixed(14)),
            97 => style.foreground = Some(ansi_term::Color::Fixed(15)),
            100 => style.background = Some(ansi_term::Color::Fixed(8)),
            101 => style.background = Some(ansi_term::Color::Fixed(9)),
            102 => style.background = Some(ansi_term::Color::Fixed(10)),
            103 => style.background = Some(ansi_term::Color::Fixed(11)),
            104 => style.background = Some(ansi_term::Color::Fixed(12)),
            105 => style.background = Some(ansi_term::Color::Fixed(13)),
            106 => style.background = Some(ansi_term::Color::Fixed(14)),
            107 => style.background = Some(ansi_term::Color::Fixed(15)),
            _ => {}
        };
        i += 1;
    }
    style
}

// Based on https://github.com/alacritty/alacritty/blob/57c4ac9145a20fb1ae9a21102503458d3da06c7b/alacritty_terminal/src/ansi.rs#L1258
fn parse_sgr_color(attrs: &[i64], i: &mut usize) -> Option<ansi_term::Color> {
    if attrs.len() < 2 {
        return None;
    }

    match attrs[*i + 1] {
        2 => {
            // RGB color spec.
            if attrs.len() < 5 {
                // debug!("Expected RGB color spec; got {:?}", attrs);
                return None;
            }

            let r = attrs[*i + 2];
            let g = attrs[*i + 3];
            let b = attrs[*i + 4];

            *i += 4;

            let range = 0..256;
            if !range.contains(&r) || !range.contains(&g) || !range.contains(&b) {
                // debug!("Invalid RGB color spec: ({}, {}, {})", r, g, b);
                return None;
            }

            Some(ansi_term::Color::RGB(r as u8, g as u8, b as u8))
        }
        5 => {
            if attrs.len() < 3 {
                // debug!("Expected color index; got {:?}", attrs);
                None
            } else {
                *i += 2;
                let idx = attrs[*i];
                match idx {
                    0..=255 => Some(ansi_term::Color::Fixed(idx as u8)),
                    _ => {
                        // debug!("Invalid color index: {}", idx);
                        None
                    }
                }
            }
        }
        _ => {
            // debug!("Unexpected color attr: {}", attrs[*i + 1]);
            None
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_iterator_1() {
        let minus_line = "\x1b[31m0123\x1b[m\n";
        assert_eq!(
            AnsiCodeIterator::new(minus_line).collect::<Vec<(Element, bool)>>(),
            vec![
                (
                    Element::Style(ansi_term::Style {
                        foreground: Some(ansi_term::Color::Red),
                        ..ansi_term::Style::default()
                    }),
                    true
                ),
                (Element::Text("0123".to_string()), false),
                (Element::Style(ansi_term::Style::default()), true),
            ]
        );
    }

    #[test]
    fn test_iterator_2() {
        let minus_line = "\x1b[31m0123\x1b[m456\n";
        assert_eq!(
            AnsiCodeIterator::new(minus_line).collect::<Vec<(Element, bool)>>(),
            vec![
                (
                    Element::Style(ansi_term::Style {
                        foreground: Some(ansi_term::Color::Red),
                        ..ansi_term::Style::default()
                    }),
                    true
                ),
                (Element::Text("0123".to_string()), false),
                (Element::Style(ansi_term::Style::default()), true),
                (Element::Text("456".to_string()), false),
            ]
        );
    }

    #[test]
    fn test_parse_first_style() {
        let minus_line_from_unconfigured_git = "\x1b[31m-____\x1b[m\n";
        let style = parse_first_style(minus_line_from_unconfigured_git);
        let expected_style = ansi_term::Style {
            foreground: Some(ansi_term::Color::Red),
            ..ansi_term::Style::default()
        };
        assert_eq!(Some(expected_style), style);
    }
}
