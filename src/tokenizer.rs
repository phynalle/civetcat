use std::collections::HashMap;
use std::rc::Rc;

use pcre::Pcre;

#[derive(Debug, Clone)]
pub enum Scope {
    Include(String),
    Match(Rc<Match>),
    Block(Rc<Block>),
}

impl Scope {
    fn name(&self) -> Option<&String> {
        match *self {
            Scope::Include(_) => None,
            Scope::Match(ref mat) => mat.name.as_ref(),
            Scope::Block(ref blk) => blk.name.as_ref(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub name: Option<String>,
    pub pat: Pattern,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub name: Option<String>,
    pub begin: Pattern,
    pub end: Pattern,
    pub subscopes: Vec<Scope>,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub pattern: String,
    pub captures: HashMap<String, String>,
}

impl Pattern {
    pub fn empty() -> Pattern {
        Pattern {
            pattern: String::new(),
            captures: HashMap::new(),
        }
    }
}

impl Pattern {
    fn find(&self, text: &str) -> Option<MatchResult> {
        let mut pcre = Pcre::compile(&self.pattern).unwrap();
        if let Some(m) = pcre.exec(text) {
            let mut captured = Vec::new();
            for i in 1..m.string_count() {
                if let Some(ref name) = self.captures.get(&i.to_string()) {
                    captured.push((i, m.group_start(i), m.group_end(i), name.to_string()));
                }
            }
            Some(MatchResult {
                start: m.group_start(0),
                end: m.group_end(0),
                captured: captured,
            })
        } else {
            None
        }
    }
}

pub struct Grammar {
    pub repository: Rc<HashMap<String, Scope>>,
    pub global: Rc<Block>,
}

pub struct Builder {
    grammar: Rc<Grammar>,
}

impl Builder {
    pub fn new(grammar: Grammar) -> Builder {
        Builder { grammar: Rc::new(grammar) }
    }

    pub fn build(&self) -> Tokenizer {
        Tokenizer::new(self.grammar.clone())
    }
}

struct MatchResult {
    start: usize,
    end: usize,
    captured: Vec<(usize, usize, usize, String)>,
}

#[derive(Debug)]
pub struct Token(pub usize, pub usize, pub String); // (begin, end, scope name)

pub struct Tokenizer {
    grammar: Rc<Grammar>,
    states: States,
}

impl Tokenizer {
    pub fn new(grammar: Rc<Grammar>) -> Tokenizer {
        Tokenizer {
            grammar: grammar,
            states: States::new(),
        }
    }

    pub fn tokenize(&mut self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        for line in text.lines() {
            // let offset = line.as_ptr() as usize - text.as_ptr() as usize;
            let mut pos = 0;
            while let Some((read_bytes, toks)) = self.tokenize_line(&line[pos..], pos) {
                tokens.extend(toks.into_iter());
                pos += read_bytes;
            }
        }
        // println!("tokens = {:?}", tokens);
        tokens
    }

    fn tokenize_line(&mut self, line: &str, offset: usize) -> Option<(usize, Vec<Token>)> {
        let (is_global, block) = if self.states.is_empty() {
            (true, self.grammar.global.clone())
        } else {
            (false, self.states.top().block.clone())
        };

        let end_matched = if !is_global {
            block.end.find(&line)
        } else {
            None
        };

        enum Matched {
            Sub(Scope),
            End,
        };

        let repos = self.grammar.repository.clone();
        let matched = block.subscopes
            .iter()
            .filter_map(|scope| {
                let scope = if let Scope::Include(ref inc) = *scope {
                    if inc.starts_with("#") {
                        repos.get(&inc[1..]).unwrap()
                    } else {
                        panic!(format!("Yet unimplemented: including : {}", inc));
                    }
                } else {
                    scope
                };
                match *scope {
                    Scope::Include(_) => panic!("Unreachable"),
                    Scope::Match(ref mat) => {
                        mat.pat.find(line).map(|m| (Matched::Sub(scope.clone()), m))
                    }
                    Scope::Block(ref blk) => {
                        blk.begin.find(line).map(|m| (Matched::Sub(scope.clone()), m))
                    }
                }
            })
            .min_by(|x, y| (x.1.start, x.1.end).cmp(&(y.1.start, y.1.end)));

        let selected = match (end_matched, matched) {
            (Some(end_matched), Some(matched)) => {
                if end_matched.start <= matched.1.start {
                    Some((Matched::End, end_matched))
                } else {
                    Some(matched)
                }
            }
            (Some(end_matched), None) => Some((Matched::End, end_matched)),
            (None, x) => x,
        };

        match selected {
            Some((Matched::End, ref m)) => {
                let mut tokens: Vec<Token> = m.captured
                    .iter()
                    .map(|&(_, start, end, ref name)| {
                        Token(offset + start, offset + end, name.clone())
                    })
                    .collect();
                if let Some(name) = block.name.as_ref() {
                    tokens.push(Token(self.states.top().pos, offset + m.end, name.clone()));
                }
                self.states.pop();
                Some((m.end, tokens))
            }
            Some((Matched::Sub(ref scope), ref m)) => {
                let mut tokens: Vec<Token> = m.captured
                    .iter()
                    .map(|&(_, start, end, ref name)| {
                        Token(offset + start, offset + end, name.clone())
                    })
                    .collect();
                if let Scope::Block(ref blk) = *scope {
                    let backref = m.captured
                        .iter()
                        .map(|&(i, begin, end, _)| (i, line[begin..end].to_string()))
                        .collect();
                    self.states.push(MatchState::new(blk.clone(), offset + m.start, backref));
                } else if let Some(name) = scope.name() {
                    tokens.insert(0, Token(offset + m.start, offset + m.end, name.clone()));
                }
                Some((m.end, tokens))
            }
            None => None,
        }
    }
}

struct States {
    matches: Vec<MatchState>,
}

impl States {
    fn new() -> States {
        States { matches: Vec::new() }
    }

    fn push(&mut self, state: MatchState) {
        self.matches.push(state);
    }

    fn pop(&mut self) {
        self.matches.pop();
    }

    fn top(&self) -> &MatchState {
        assert!(!self.matches.is_empty());
        &self.matches[self.matches.len() - 1]
    }

    fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

struct MatchState {
    block: Rc<Block>,
    pos: usize,
    captured: HashMap<usize, String>,
}

impl MatchState {
    fn new(block: Rc<Block>, pos: usize, captured: HashMap<usize, String>) -> MatchState {
        MatchState {
            block: block,
            pos: pos,
            captured: captured,
        }
    }
}
