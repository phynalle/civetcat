use std::io::Result;
use std::rc::Rc;
use syntax::rule::{self, CaptureGroup, Compiler, Rule, RuleId, Type};
use syntax::regex::{self, Regex};
use syntax::str_piece::StrPiece;

pub fn load_grammar_from_source(src_name: &str) -> Result<Grammar> {
    let mut c = Compiler::new(src_name);
    Ok(c.compile())
}

pub struct Grammar {
    root_id: RuleId,
    rules: Vec<Rule>,
}

impl Grammar {
    pub fn new(rules: Vec<Rule>, root_id: RuleId) -> Grammar {
        Grammar { root_id, rules }
    }

    pub fn rule(&self, id: RuleId) -> Rule {
        self.rules[id].clone()
    }
}

enum BestMatchResult {
    Pattern(rule::MatchResult),
    End(regex::MatchResult),
    None,
}

pub struct Tokenizer {
    state: State,
    grammar: Rc<Grammar>,
    tokengen: TokenGenerator,
}

impl Tokenizer {
    pub fn new(grammar: Rc<Grammar>) -> Tokenizer {
        let mut tokenizer = Tokenizer {
            state: State::new(),
            grammar: Rc::clone(&grammar),
            tokengen: TokenGenerator::new(),
        };
        tokenizer.state.push(grammar.rule(grammar.root_id), None);
        tokenizer
    }

    // pub fn tokenize(&mut self, s: &str) -> Vec<Vec<Token>> {
    //     let mut line_tokens = Vec::new();
    //     for line in s.lines() {
    //         line_tokens.push(self.tokenize_line(line));
    //     }
    //     line_tokens
    // }

    pub fn tokenize_line(&mut self, line: &str) -> Vec<Token> {
        let line_str = StrPiece::new(line);
        let while_not_matched = {
            let top = self.state.top();
            match top.rule.display() {
                Type::BeginWhile => top.match_expr(line_str).is_none(),
                _ => false,
            }
        };
        if while_not_matched {
            self.state.pop();
        }

        self.tokenize_string(line_str);

        let tokens: Vec<Token> = self.tokengen.tokens.drain(..).collect();
        self.tokengen = TokenGenerator::new();
        tokens
    }

    fn tokenize_string<'b>(&mut self, mut text: StrPiece<'b>) {
        loop {
            match self.tokenize_next(text) {
                Some(pos) => text.remove_prefix(pos),
                None => break,
            }
        }
    }

    fn tokenize_next<'b>(&mut self, text: StrPiece<'b>) -> Option<usize> {
        match self.best_match(text) {
            BestMatchResult::Pattern(m) => {
                let pos = (m.caps.start() + text.start(), m.caps.end() + text.start());
                self.generate_token(pos.0);

                // TOOD: remove repetitive codes below
                let rule = self.grammar.rule(m.rule);
                rule.do_match(|r| {
                    self.state.push(rule.clone(), None);
                    self.process_capture(text, &m.caps.captures, &r.captures);
                    self.generate_token(pos.1);
                    self.state.pop();
                });
                rule.do_beginend(|r| {
                    let s = replace_backref(r.end_expr.clone(), text, &m.caps);
                    self.state.push(rule.clone(), Some(s));
                    self.process_capture(text, &m.caps.captures, &r.begin_captures);
                    self.generate_token(pos.1);
                });
                rule.do_beginwhile(|r| {
                    let s = replace_backref(r.while_expr.clone(), text, &m.caps);
                    self.state.push(rule.clone(), Some(s));
                    self.process_capture(text, &m.caps.captures, &r.begin_captures);
                    self.generate_token(pos.1);
                });
                Some(m.caps.end())
            }
            BestMatchResult::End(m) => {
                let pos = (m.start() + text.start(), m.end() + text.start());
                self.generate_token(pos.0);
                {
                    let rule = self.state.top().rule.clone();
                    rule.do_beginend(|r| {
                        self.process_capture(text, &m.captures, &r.end_captures);
                    });
                }
                self.generate_token(pos.1);
                self.state.pop();
                Some(m.end())
            }
            BestMatchResult::None => {
                self.generate_token(text.end());
                None
            }
        }
    }

    fn done(&self) {
        assert!(self.state.depth() == 1);
    }

    fn best_match<'b>(&mut self, text: StrPiece<'b>) -> BestMatchResult {
        let state = self.state.top();
        let pattern_match = state
            .rule
            .match_subpatterns(text)
            .into_iter()
            .filter(|x| x.caps.start() != x.caps.end())
            .min_by_key(|x| x.caps.start());
        let end_match = match state.rule.display() {
            Type::BeginEnd => state.match_expr(text),
            _ => None,
        };

        match (pattern_match, end_match) {
            (None, None) => BestMatchResult::None,
            (None, Some(e)) => BestMatchResult::End(e),
            (Some(p), None) => BestMatchResult::Pattern(p),
            (Some(p), Some(e)) => {
                if e.start() <= p.caps.start() {
                    BestMatchResult::End(e)
                } else {
                    BestMatchResult::Pattern(p)
                }
            }
        }
    }

    fn process_capture<'b>(
        &mut self,
        text: StrPiece<'b>,
        captured: &[Option<(usize, usize)>],
        capture_group: &CaptureGroup,
    ) {
        for (i, cap) in captured.into_iter().enumerate() {
            if let Some(pos) = *cap {
                if let Some(rule) = capture_group.0.get(&i) {
                    let captured_text = text.substr(pos.0, pos.1 - pos.0);
                    self.generate_token(captured_text.start());
                    self.state.push(rule.upgrade().unwrap(), None);
                    self.tokenize_string(captured_text);
                    self.state.pop();
                }
            }
        }
    }

    fn generate_token(&mut self, pos: usize) {
        self.tokengen.generate(pos, &self.state)
    }
}

struct RuleState {
    rule: Rule,
    expr: Option<Regex>,
}

impl RuleState {
    fn match_expr<'a>(&self, text: StrPiece<'a>) -> Option<regex::MatchResult> {
        self.expr.as_ref().and_then(|expr| expr.find(text))
    }
}

struct State {
    st: Vec<RuleState>,
    scopes: Vec<Option<String>>,
}

impl State {
    fn new() -> State {
        State {
            st: Vec::new(),
            scopes: Vec::new(),
        }
    }

    fn top(&self) -> &RuleState {
        assert!(!self.st.is_empty());
        self.st.iter().rev().nth(0).unwrap()
    }

    fn push(&mut self, rule: Rule, expr: Option<String>) {
        self.st.push(RuleState {
            rule: rule.clone(),
            expr: expr.map(|s| Regex::new(&s)),
        });
        self.scopes.push(rule.name());
    }

    fn pop(&mut self) {
        assert!(!self.st.is_empty());
        self.st.pop();
        self.scopes.pop();
    }

    #[allow(dead_code)]
    fn depth(&self) -> usize {
        self.st.len()
    }
}

struct TokenGenerator {
    pos: usize,
    tokens: Vec<Token>,
}

impl TokenGenerator {
    fn new() -> TokenGenerator {
        TokenGenerator {
            pos: 0,
            tokens: Vec::new(),
        }
    }

    fn generate(&mut self, end: usize, state: &State) {
        if self.pos < end {
            let token = self.generate_token(end, state);
            self.tokens.push(token);
        }
    }

    fn generate_token(&mut self, end: usize, state: &State) -> Token {
        assert!(self.pos < end);
        let start = self.pos;
        self.pos = end;
        Token {
            start,
            end,
            scopes: state.scopes.iter().filter_map(|s| s.clone()).collect(),
        }
    }
}

#[derive(Debug)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub scopes: Vec<String>,
}

fn replace_backref<'a>(mut s: String, text: StrPiece<'a>, m: &regex::MatchResult) -> String {
    for (i, cap) in m.captures.iter().enumerate().skip(1) {
        if let Some(ref cap) = *cap {
            let old = format!("\\{}", i);
            let new = text.substr(cap.0, cap.1 - cap.0);
            s = s.replace(&old, &new);
        }
    }
    s
}
