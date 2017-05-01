use std::marker::PhantomData;

pub struct Parser<'a, T>
    where T: 'a + Tokenizer<'a, T>
{
    tokenizer: T,
    _ghost: PhantomData<&'a T>,
}

impl<'a, T> Parser<'a, T>
    where T: Tokenizer<'a, T>
{
    pub fn new(text: &'a str) -> Parser<'a, T> {
        Parser {
            tokenizer: T::with_text(text),
            _ghost: PhantomData,
        }
    }

    pub fn parse_next(&mut self) -> Option<Token<'a>> {
        self.tokenizer.next()
    }
}

pub enum TokenType {
    Keyword,
    String,
    Number,
    Comment,
    PlainText,
}

pub struct Token<'a> {
    pub ttype: TokenType,
    pub start: usize,
    pub end: usize,
    pub text: &'a str,
}

pub trait Tokenizer<'a, T: 'a> {
    fn with_text(&'a str) -> T;
    fn next(&mut self) -> Option<Token<'a>>;
}

pub struct SimpleTokenizer<'a> {
    text: &'a str,
    pos: usize,
    len: usize,
}

impl<'a> SimpleTokenizer<'a> {
    fn parse_next(&mut self) -> Option<Token<'a>> {
        if self.pos >= self.len {
            return None;
        }

        let (ttype, len) = if let Some(n) = self.parse_comment() {
            (TokenType::Comment, n)
        } else if let Some(n) = self.parse_string() {
            (TokenType::String, n)
        } else if let Some((ttype, n)) = self.parse_word() {
            (ttype, n)
        } else if let Some(n) = self.parse_number() {
            (TokenType::Number, n)
        } else {
            (TokenType::PlainText, 1)
        };

        let (start, end) = (self.pos, self.pos + len);
        self.pos += len;
        Some(Token {
            ttype: ttype,
            start: start,
            end: end,
            text: &self.text[start..end],
        })
    }

    fn parse_word(&self) -> Option<(TokenType, usize)> {
        // keywords list would be better if it could be allocated as static
        let mut keywords = vec![
            "abstract", "alignof", "as", "become", "box", "break", "const",
            "continue", "crate", "do", "else","enum", "extern", "false",
            "final", "fn", "for", "if", "impl", "in", "let", "loop", "macro",
            "match", "mod", "move", "mut", "offsetof", "override", "priv", "proc",
            "pub", "pure", "ref", "return", "Self", "self", "sizeof", "static", "struct",
            "super", "trait", "true", "type", "typeof", "unsafe", "unsized",
            "use", "virtual", "where", "while", "yield",
            "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32", "f64",
            "Some", "None", "Ok", "Err", "Option", "Result", "Vec", "Box", "String",
        ];
        keywords.sort();

        let word: String = self.text
            .chars()
            .skip(self.pos)
            .take_while(|c| c.is_alphabetic() || c.is_digit(10))
            .collect::<_>();

        if word.is_empty() {
            None
        } else {
            if (word.as_bytes()[0] as char).is_digit(10) {
                return None;
            }
            let ttype = if let Ok(_) = keywords.binary_search(&word.as_str()) {
                TokenType::Keyword
            } else {
                TokenType::PlainText
            };
            Some((ttype, word.len()))
        }
    }

    fn parse_string(&self) -> Option<usize> {
        let text = self.text.as_bytes();
        let mut pos = self.pos;

        let open = if pos < self.len && (text[pos] == b'\'' || text[pos] == b'\"') {
            let b = text[pos];
            pos += 1;
            b

        } else {
            return None;
        };
        if open == b'\'' {
            if pos + 1 < self.len && text[pos + 1] == b'\\' {
                pos += 1;
            }
            if self.len <= pos + 2 || text[pos + 2] != b'\'' {
                return None;
            }
            pos += 2;

        } else {
            while pos < self.len {
                if text[pos] == open {
                    if text[pos - 1] != b'\\' {
                        pos += 1;
                        break;
                    }
                }
                pos += 1;
            }
        }

        let len = pos - self.pos;
        if len == 0 { None } else { Some(len) }
    }

    fn parse_number(&self) -> Option<usize> {
        let len = self.text.chars().skip(self.pos).take_while(|c| c.is_digit(10)).count();
        if len == 0 { None } else { Some(len) }
    }

    fn parse_comment(&self) -> Option<usize> {
        if self.len - self.pos < 2 {
            return None;
        }

        let mut pos = self.pos;
        let op = &self.text[pos..pos + 2];
        let text = &self.text;

        if op == "//" {
            let len = self.text.chars().skip(pos).take_while(|&c| c != '\n').count();
            Some(len)
        } else if op == "/*" {
            pos += 2;
            while pos < self.len {
                if &text[pos - 2..pos] == "*/" {
                    break;
                }
                pos += 1;
            }
            Some(pos - self.pos)
        } else {
            None
        }
    }
}

impl<'a> Tokenizer<'a, SimpleTokenizer<'a>> for SimpleTokenizer<'a> {
    fn with_text(text: &'a str) -> SimpleTokenizer {
        SimpleTokenizer {
            text: text,
            pos: 0,
            len: text.len(),
        }
    }

    fn next(&mut self) -> Option<Token<'a>> {
        self.parse_next()
    }
}
