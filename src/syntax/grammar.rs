use std::io::Result;
use serde_json;
use syntax::rule::{self, Compiler, Rule, RuleId, RawRule, CaptureGroup};
use syntax::regex::{self, Regex};

pub fn load_grammar(raw_text: &str) -> Result<Grammar> {
    let mut rule: RawRule = serde_json::from_str(raw_text)?;
    let mut c = Compiler::new();
    Ok(c.compile(&mut rule))
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

pub struct Tokenizer<'a> {
    state: State,
    grammar: &'a Grammar,
    tokengen: TokenGenerator,
}

impl<'a> Tokenizer<'a> {
    pub fn new(grammar: &'a Grammar) -> Tokenizer {
        let mut tokenizer = Tokenizer {
            state: State::new(),
            grammar,
            tokengen: TokenGenerator::new(),
        };
        tokenizer.state.push(grammar.rule(grammar.root_id), None);
        tokenizer
    }

    pub fn tokenize(&mut self, s: &str) -> Vec<Vec<Token>> {
        let mut line_tokens = Vec::new();
        for line in s.lines() {
            line_tokens.push(self.tokenize_line(line));
        }
        line_tokens
    }

    pub fn tokenize_line(&mut self, line: &str) -> Vec<Token> {
        self.tokenize_string(line, 0);

        let tokens: Vec<Token> = self.tokengen.tokens.drain(..).collect();
        self.tokengen = TokenGenerator::new();
        tokens
    }

    fn tokenize_string(&mut self, text: &str, offset: usize) {
        let n = text.len();
        let mut p = 0;
        while p < n {
            match self.tokenize_next(&text[p..], offset + p) {
                Some(pos) => p = pos - offset,
                None => break,
            }
        }
    }

    fn tokenize_next(&mut self, text: &str, offset: usize) -> Option<usize> {
        match self.best_match(text) {
            BestMatchResult::Pattern(m) => {
                let pos = (m.caps.start() + offset, m.caps.end() + offset);
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
                }
                self.state.pop();
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
            |expr| expr.find(text)
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
                if let Some(rule_id) = capture_group.0.get(&i) {
                    self.state.push(self.grammar.rule(*rule_id), None);
                    if self.tokengen.pos < pos.0 {
                        self.tokengen.generate(pos.0, &self.state);
                    }
                    self.tokenize_string(&text[pos.0..pos.1], offset + pos.0);
                    self.state.pop();
                }
            }
        }

    }
}

struct RuleState {
    rule: Rule,
    end_expr: Option<Regex>,
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
            end_expr: expr.map(|s| Regex::new(&s)),
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

pub struct Token {
    pub start: usize,
    pub end: usize,
    pub scopes: Vec<String>,
}
