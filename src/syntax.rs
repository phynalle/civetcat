use std::io::Result;
use std::fs::File;
use std::collections::HashMap;

use pcre::Pcre;
use serde_json;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Pattern {
    Root(Syntax),
    Include(Include),
    Match(Match),
    Block(Block),
}

impl Pattern {
    fn refer<'a, 'b: 'a>(&'b self, repos: &'a HashMap<String, Pattern>) -> &'a Pattern {
        match *self {
            Pattern::Include(ref p) => p.refer(&repos),
            _ => &self,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Include {
    include: String,
}
impl Include {
    fn name(&self) -> String {
        if self.include.starts_with("#") {
            self.include.chars().skip(1).collect()
        } else {
            self.include.clone()
        }
    }

    fn refer<'a>(&self, repos: &'a HashMap<String, Pattern>) -> &'a Pattern {
        let root = self.name();
        let mut current = root.clone();
        loop {
            let pattern = repos.get(&current).unwrap();
            match *pattern {
                Pattern::Include(ref p) => {
                    let target = p.name();
                    if target == root {
                        panic!("Cycle Error");
                    }
                    current = target;
                }
                Pattern::Match(_) |
                Pattern::Block(_) => {
                    return &pattern;
                }
                _ => panic!("Unreachable"),
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Block {
    #[serde(rename = "name")]
    scope: Option<String>,
    begin: String,
    end: String,
    begin_captures: Captures,
    end_captures: Captures,
    patterns: Option<Vec<Pattern>>,
}

impl Block {
    fn match_begin<'a>(&self, cursor: &mut TextCursor) -> Option<Vec<Token>> {
        let mut pcre = Pcre::compile(&self.begin).unwrap();
        if let Some(m) = pcre.exec(cursor.text()) {
            let pos = cursor.orig_pos;

            let mut tokens = Vec::new();
            let mut captures = Vec::new();
            if let Some(ref scope) = self.scope {
                captures.push((pos + m.group_start(0), pos + m.group_end(0), scope.clone()));
            }
            if let Some(ref caps) = self.begin_captures {
                for i in 1..m.string_count() {
                    if let Some(ref cap) = caps.get(&i.to_string()) {
                        captures.push((pos+m.group_start(i), pos+m.group_end(i), cap.name.clone()));
                    }
                }
            }
            tokens.push(Token {
                text: m.group(0).to_string(),
                captures: captures,
            });
            cursor.consume(m.group_end(0));
            Some(tokens)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Match {
    #[serde(rename = "name")]
    scope: Option<String>,
    #[serde(rename = "match")]
    pattern: String,
    captures: Captures,
}

impl Match {
    fn tokenize<'a>(&self, cursor: &mut TextCursor) -> Option<Vec<Token>> {
        let mut pcre = Pcre::compile(&self.pattern).unwrap();
        if let Some(m) = pcre.exec(cursor.text()) {
            let mut tokens = Vec::new();
            let pos = cursor.orig_pos;
            let mut captures = Vec::new();
            if let Some(ref scope) = self.scope {
                captures.push((pos + m.group_start(0), pos + m.group_end(0), scope.clone()));
            }
            if let Some(ref caps) = self.captures {
                for i in 1..m.string_count() {
                    if let Some(ref cap) = caps.get(&i.to_string()) {
                        captures.push((pos+m.group_start(i), pos+m.group_end(i), cap.name.clone()));
                    }
                }
            }
            tokens.push(Token {
                text: m.group(0).to_string(),
                captures: captures,
            });
            cursor.consume(m.group_end(0));
            Some(tokens)
        } else {
            None
        }
    }
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Syntax {
    name: String,
    scope_name: String,
    file_types: Vec<String>,
    patterns: Vec<Pattern>,
    repository: HashMap<String, Pattern>,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Capture {
    name: String,
}

type Captures = Option<HashMap<String, Capture>>;

struct TextCursor<'a> {
    text: &'a str,
    pos: usize,

    orig: &'a str,
    orig_pos: usize,
}

impl<'a> TextCursor<'a> {
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    fn consume(&mut self, n: usize) {
        self.pos += n;
        self.orig_pos += n;
    }

    fn text(&self) -> &'a str {
        &self.text[self.pos..].lines().nth(1).unwrap_or(&self.text[self.pos..])
    }
}


pub struct Tokenizer {
    root: Pattern,
}

impl Tokenizer {
    pub fn create(filename: &str) -> Result<Tokenizer> {
        let file = File::open(filename)?;
        let syntax: Syntax = serde_json::from_reader(file).unwrap();
        let root = Pattern::Root(syntax);
        
        Ok(Tokenizer {
            root: root,
        })
    }

    pub fn instance<'a>(&'a self) -> RegexTokenizer<'a> {
        RegexTokenizer::new(&self.root)
    }
}

pub struct RegexTokenizer<'a> {
    root: &'a Pattern,
    stack: Stack<'a>,
}

impl<'a> RegexTokenizer<'a> {
    fn new(root: &'a Pattern) -> RegexTokenizer<'a> {
        RegexTokenizer {
            root: root,
            stack: Stack::new(),
        }
    }

    pub fn tokenize(&mut self, text: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        self.stack.push(&self.root, 0);

        // let mut stack = Stack::new(&root);
        for line in text.lines() {
            let pos = line.as_ptr() as usize - text.as_ptr() as usize;
            let len = line.len();

            let mut cursor = TextCursor {
                text: line,
                pos: 0,
                orig: text,
                orig_pos: pos,
            };

            while cursor.pos < len {
                let result = self.tokenize_line(&mut cursor);
                if result.is_none() {
                    break;
                }
                if let Some(toks) = result {
                    tokens.extend_from_slice(&toks);
                }
            }
        }
        tokens
    }

    fn repository(&self) -> &'a HashMap<String, Pattern> {
        if let Pattern::Root(ref r) = *self.root {
            &r.repository
        } else {
            panic!("Unreachable!!!");
        }
    }

    fn tokenize2<'b>(&mut self,
                     patterns: &'a Vec<Pattern>,
                     mut cursor: &mut TextCursor)
                     -> Option<Vec<Token>> {
        for pat in patterns {
            let pat = pat.refer(self.repository());
            if let &Pattern::Match(ref p) = pat {
                let result = p.tokenize(&mut cursor);
                if result.is_none() {
                    continue;
                }
                return result;
            } else if let &Pattern::Block(ref p) = pat {
                let result = p.match_begin(&mut cursor);
                if result.is_none() {
                    continue;
                }
                self.stack.push(&pat, cursor.orig_pos);
                return result;
            }
        }
        None
    }

    fn tokenize_line<'b>(&mut self, mut cursor: &mut TextCursor) -> Option<Vec<Token>> {
        match *self.stack.top().0 {
            Pattern::Block(ref r) => {


                if let Some(ref pats) = r.patterns {
                    let result = self.tokenize2(&pats, &mut cursor);
                    if result.is_some() {
                        return result;
                    }
                    // if let Some(ref toks) = result {
                    // tokens.extend_from_slice(&toks);
                    // }
                }


            }
            Pattern::Root(ref r) => {
                return self.tokenize2(&r.patterns, &mut cursor);
            }
            _ => panic!("Unreachable!"),
        };

        if let Pattern::Block(ref r) = *self.stack.top().0 {
            let mut tokens = Vec::new();
            if let Some(m) = Pcre::compile(&r.end).unwrap().exec(cursor.text()) {
                if let Some(scope) = self.stack.top_scope() {
                    let pos_begin = self.stack.top().1;
                    let pos_end = cursor.orig_pos + m.group_end(0);

                    let token = Token {
                        text: String::from(&cursor.orig[pos_begin..pos_end]),
                        captures: vec![(pos_begin, pos_end, scope)],
                    };
                    tokens.push(token);
                    cursor.consume(m.group_end(0));
                }
                self.stack.pop();
                return Some(tokens);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    text: String,
    pub captures: Vec<(usize, usize, String)>,
}

// struct Repository {
//     inner: HashMap<String, Rc<CompiledPattern>>
// }

// impl Repository {
//     fn new(base: &HashMap<String, Pattern>) {
//         for (name, pattern) in base {

//         }

//     }
// }

// struct Ruler {
//     repos: HashMap<String, CompiledPattern>, // name, pattern
// }

// #[derive(Debug)]
// struct PatternMap {
//     names: Vec<Pattern>,
//     re: RegexSet,
// }

struct Stack<'a> {
    scopes: Vec<(&'a Pattern, usize)>,
}

impl<'a> Stack<'a> {
    fn new() -> Stack<'a> {
        Stack { scopes: vec![] }
    }

    fn push(&mut self, pat: &'a Pattern, pos: usize) {
        self.scopes.push((pat, pos));
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn top(&self) -> (&'a Pattern, usize) {
        self.scopes[self.scopes.len() - 1]
    }

    fn top_scope(&self) -> Option<String> {
        if let (&Pattern::Block(ref r), _) = self.top() {
            r.scope.clone()
        } else {
            None
        }
    }
}
