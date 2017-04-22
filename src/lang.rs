use std::io::{Read, Cursor};

use parser::{Parser, SimpleTokenizer, TokenType};

/*
struct Regexes {
    conds: Vec<(String, String, String)>,
    re: Option<Regex>,
    replace_expr: String,
}

impl Regexes {
    fn new() -> Regexes {
        Regexes {
            conds: Vec::new(),
            re: None,
            replace_expr: String::new(),
        } 
    }

    fn insert<S: AsRef<str>>(&mut self, keyword: S, expr: S, color: u8) {
        let expr = format!("(?P<{}>{})", keyword.as_ref(), expr.as_ref());
        let replace_expr = format!("{}${}{}", colorize(color), keyword.as_ref(), colorize(0));

        self.conds.push(
            (keyword.as_ref().to_owned(), 
             // Regex::new(expr.as_str()).unwrap(), 
             replace_expr,
             expr)
        );
    }

    fn build(&mut self) {
        let expr = self.conds.iter()
            .map(|cond| cond.2.clone())
            .collect::<Vec<String>>()
            .join("|");
        let replace_expr = self.conds.iter()
            .map(|cond| cond.1.clone())
            .collect::<Vec<String>>()
            .concat();

        self.re = Some(Regex::new(&expr).unwrap());
        self.replace_expr = replace_expr;
    }

    fn apply(&self, text: String) -> String {
        if let Some(ref re) = self.re {
            re.replace_all(text.as_str(), self.replace_expr.as_str()).to_string()
        } else {
            panic!("regex is not built");
        }
    }
}
*/

pub struct Highlighter;

impl Highlighter {
    /* pub fn new() -> Highlighter {
        let mut regexes = Regexes::new();

        let keywords = ["if", "else", "for", "loop", "while", "fn", "struct", "impl", "use", "trait", "mod", "extern", "crate", "match", "let"];
        let keyword_expr = keywords.iter().map(|s| [r"\b", s, r"\b"].concat()).collect::<Vec<_>>().join("|");

        let string_expr = "\"([^\"]*)\"".to_owned();
        let comment = "(//.*)".to_owned();

        regexes.insert("comment", &comment, 35);
        regexes.insert("string", &string_expr, 33);
        regexes.insert("keyword", &keyword_expr, 34);
        regexes.build();

        Highlighter {
            re: regexes
        }
    } */

    pub fn apply<R>(&self, mut reader: R) -> Cursor<String> where R: Read {
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
            
            let s = 
                if is_colored {
                    // it would be problem when return carrage is '\r\n'
                    token.text.lines()
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