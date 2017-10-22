use std::io::Result;
use std::fs::File;
use std::cell::RefCell;

use serde_json;

use syntax::rule::{self, Rule, RuleId, RawRule, Capture, CaptureGroup};
use syntax::regex::{self, simple_match};

pub fn load_grammars(path: &str) -> Result<RawRule> {
    let f = File::open(path)?;
    let r: RawRule = serde_json::from_reader(f)?;
    Ok(r)
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

    pub fn tokenize_test(&self, s: &str) {
        let mut tokenizer = Tokenizer::new(&self);
        tokenizer.tokenize(s);
    }
}

enum BestMatchResult {
    Pattern(rule::MatchResult),
    End(regex::MatchResult),
    None,
}

struct Tokenizer<'a> {
    state: State,
    grammar: &'a Grammar,
    tokengen: TokenGenerator,
}

impl<'a> Tokenizer<'a> {
    fn new(grammar: &'a Grammar) -> Tokenizer {
        let mut t = Tokenizer {
            state: State::new(),
            grammar,
            tokengen: TokenGenerator::new(),
        };
        t.state.push(grammar.rule(0), None);
        t
    }

    pub fn tokenize(&mut self, s: &str) {
        for line in s.lines() {
            self.tokenize_line(line);
            for token in &self.tokengen.tokens {
                println!(
                    "({}, {}): {:?}, {}",
                    token.start,
                    token.end,
                    token.scopes,
                    &line[token.start..token.end]
                );
            }
            self.tokengen = TokenGenerator::new();
        }
    }

    fn tokenize_line(&mut self, line: &str) {
        let n = line.len();
        let mut offset = 0;
        while offset < n {
            match self.tokenize_next(&line[offset..n], offset) {
                Some(pos) => offset = pos,
                None => break,
            }
        }
    }

    fn tokenize_next(&mut self, text: &str, offset: usize) -> Option<usize> {
        let state: RuleState;

        match self.best_match(text) {
            BestMatchResult::Pattern(m) => {
                let pos = (m.caps.start() + offset, m.caps.end() + offset);
                // println!("[p]pos: {:?}", pos);
                if self.tokengen.pos < pos.0 {
                    self.tokengen.generate(pos.0, &self.state);
                }

                let rule = self.grammar.rule(m.rule);
                rule.do_match(|r| {
                    self.state.push(rule.clone(), None);
                    self.process_capture(text, offset, &m.caps.captures, &r.captures);
                    self.tokengen.generate(pos.1, &self.state);
                    self.state.pop();
                });
                rule.do_beginend(|r| {
                    self.state.push(rule.clone(), Some(r.end_expr.clone()));
                    self.process_capture(text, offset, &m.caps.captures, &r.begin_captures);
                    self.tokengen.generate(pos.1, &self.state);
                });

                Some(pos.1)
            }
            BestMatchResult::End(m) => {
                let pos = (m.start() + offset, m.end() + offset);
                if self.tokengen.pos < pos.0 {
                    self.tokengen.generate(pos.0, &self.state);
                }
                {
                    let rule = self.state.current().rule.clone();
                    rule.do_beginend(|r| {
                        self.process_capture(text, offset, &m.captures, &r.end_captures);
                    });
                } self.state.pop();
                self.tokengen.generate(pos.1, &self.state);

                Some(pos.1)
            }
            BestMatchResult::None => {
                self.tokengen.generate(offset + text.len(), &self.state);
                None
            }
        }
    }

    fn best_match(&mut self, text: &str) -> BestMatchResult {
        let state = self.state.current();
        let pattern_match = state
            .rule
            .match_subpatterns(text, &self.grammar.rules)
            .into_iter()
            .min_by_key(|x| x.caps.start());
        let end_match = state.end_expr.as_ref().and_then(
            |expr| simple_match(expr, text),
        );

        match (pattern_match, end_match) {
            (None, None) => BestMatchResult::None,
            (None, Some(e)) => BestMatchResult::End(e),
            (Some(p), None) => BestMatchResult::Pattern(p),
            (Some(p), Some(e)) => {
                if e.start() < p.caps.start() {
                    BestMatchResult::End(e)
                } else {
                    BestMatchResult::Pattern(p)
                }
            }
        }
    }

    fn process_capture(
        &mut self,
        text: &str,
        offset: usize,
        captured: &[Option<(usize, usize)>],
        capture_group: &CaptureGroup,
    ) {
        for (i, cap) in captured.into_iter().enumerate() {
            if let Some(pos) = *cap {
                if let Some(capture) = capture_group.0.get(&i) {
                    if capture.rule_id.is_some() {
                        self.state.push(
                            self.grammar.rule(capture.rule_id.unwrap()),
                            None,
                        );
                        if self.tokengen.pos < pos.0 {
                            self.tokengen.generate(pos.0, &self.state);
                        }
                        self.tokenize_next(&text[pos.0..pos.1], offset + pos.0);
                        self.state.pop();
                    }
                }
            }
        }

    }
}

struct RuleState {
    rule: Rule,
    end_expr: Option<String>,
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

    fn current(&self) -> &RuleState {
        assert!(!self.st.is_empty());
        self.st.iter().rev().nth(0).unwrap()
    }

    fn push(&mut self, rule: Rule, expr: Option<String>) {
        self.st.push(RuleState {
            rule: rule.clone(),
            end_expr: expr,
        });
        self.scopes.push(rule.name());
    }

    fn pop(&mut self) {
        assert!(!self.st.is_empty());
        self.st.pop();
        self.scopes.pop();
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

struct Token {
    start: usize,
    end: usize,
    scopes: Vec<String>,
}
