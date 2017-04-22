use std::io::{Read, Cursor};

use parser::{Parser, SimpleTokenizer, TokenType};

pub struct Highlighter;

impl Highlighter {
    pub fn apply<R>(&self, mut reader: R) -> Cursor<String>
        where R: Read
    {
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        let mut replaced = String::with_capacity(buf.len());
        let mut parser: Parser<SimpleTokenizer> = Parser::new(&buf);
        while let Some(token) = parser.parse_next() {
            let (is_colored, n) = match token.ttype {
                TokenType::Keyword => (true, 34),
                TokenType::String => (true, 33),
                TokenType::Number => (true, 31),
                TokenType::Comment => (true, 32),
                _ => (false, 0),
            };

            let s = if is_colored {
                // it would be problem when return carrage is '\r\n'
                token.text
                    .lines()
                    .map(|s| format!("{}{}{}", colorize(n), s, colorize(0)))
                    .collect::<Vec<String>>()
                    .join("\n")

            } else {
                format!("{}", token.text)
            };
            replaced.push_str(&s);
        }
        Cursor::new(replaced)
    }
}

fn colorize(color: u8) -> String {
    format!("{}[{}m", String::from_utf8(vec![27]).unwrap(), color)
}
