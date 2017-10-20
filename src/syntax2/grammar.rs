use std::io::Result;
use std::fs::File;

use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::{Ref, RefCell};

use serde_json;

use syntax2::rule::{MatchResult, RawRule, Rule, RuleId, BeginEndRule, CaptureGroup};
use syntax2::str_piece::StrPiece;
use syntax2::regex_set::{MatchResult, RegexSet};

/* struct Repository {
    inner: HashMap<String, RuleId>, } 
impl Repository {
    fn new() -> Repository {
        Repository {
            inner: HashMap::new(),
        }
    }

} */

pub fn load_grammars(path: &str) -> Result<RawRule> {
    let f = File::open(path)?;
    let r: RawRule = serde_json::from_reader(f)?;
    Ok(r)
}

enum MatchType<T, E> {
    Match(T),
    End,
}

pub struct Grammar {
    rules: Vec<Rule>,
    root: Rule,
}

impl Grammar {
    pub fn new(rules: Vec<Rule>, root_id: RuleId) -> Grammar {
        let root = rules[root_id].clone();
        Grammar { rules, root }
    }

    fn rule(&self, id: RuleId) -> Rule {
        self.rules[id].clone()
    }

    pub fn tokenize_test(&self, s: &str) {}

    fn tokenize_string<'a>(&self, text: StrPiece<'a>, states: &mut States, token_producer: TokenProducer<'a>) {
        let current_state = states.top();

        loop {
            if let Some(matched_result) = self.find_match(states) {
            } else {
                let exit_result = current_state .end_rule() .map(|sources| sources.find(text.get())); states.pop();
            }
        }
    }

    fn tokenize_string_once<'a>(&self, text: StrPiece<'a>) -> bool {
       true 
    }

    fn find_match<'a>(text: StrPiece<'a>, states: &mut States) -> Option<MatchResult> {
        current_state
            .rule()
            .search(text.get(), &self.rules)
            .min_by_key(|x, y| x.start.cmp(y))
            .map(|m| (m.start, m.end, m.groups));

    }
}

enum Reason {
    InPattern,
    End,
}

pub struct TokenResult {
    tokens: Vec<Token>,
}

struct Token {
    start: usize,
    end: usize,
    scope: String,
}


struct States {
    scopes: ScopePile,
    inner: Vec<State>,
}

impl States {
    fn new() -> States {
        States {
            scopes: ScopePile::new(),
            inner: Vec::new(),
        }
    }

    fn top(&self) -> &State {
        assert!(!self.inner.is_empty());
        &self.inner.last().unwrap()
    }

    fn push_state(&mut self, rule: Rule) -> &State {
        self.inner.push(State::new(rule));
        self.top()
    }

    fn pop_state(&mut self) {
        self.inner.pop();
    }

}

struct State {
    rule: Rule,
    end_rule: Option<RegexSet>,
    depth: usize,
}

impl State {
    fn new(rule: Rule) -> State {
        State {
            rule,
            end_rule: None,
            depth: 0,
        }
    }

    fn rule(&self) -> Rule {
        self.rule.clone() 
    }

    fn set_exit_matcher(&mut self, rule: &BeginEndRule) {
        self.end_rule = Some(RegexSet::with_patterns(&[&rule.end]));
    }

    fn end_rule(&self) -> Option<&RegexSet> {
        self.end_rule.as_ref()
    }

}

struct ScopePile {
    inner: Vec<String>,
}

impl ScopePile {
    fn new() -> ScopePile {
        ScopePile { inner: Vec::new() }
    }

    fn collect(&self) -> Vec<String> {
        self.inner.clone()
    }

    fn push(&mut self, scope: String) {
        self.inner.push(scope);
    }

    fn pop(&mut self) {
        self.inner.pop();
    }
}

struct TokenProducer<'a> {
    line: &'a str,
    last_pos: usize,
    tokens: Vec<Token>,
}

impl<'a> TokenProducer<'a> {
    fn new(line: &'a str) -> TokenProducer {
        TokenProducer {
            line,
            last_pos: 0,
            tokens: Vec::new(),
        } 
    }

    fn produce(&mut self, scope: String, end_index: usize) {
        self.produce_token(scope, end_index, false)
    }
    
    fn produce_token(&mut self, scope: String, end: usize, from_first: bool) {
        let start = match from_first {
            true => 0,
            false if self.last_pos < end => self.last_pos,
            _ => return,
        };
        self.tokens.push(Token {
            start, 
            end,
            scope: scope,
        });
        self.last_pos = end;
    }

    fn get_result(mut self) -> Vec<Token> {
        let length = self.line.len();
        if self.tokens.last().is_some() && self.tokens.last().unwrap().start == length -1 { 
            self.tokens.pop();
        }
        if self.tokens.is_empty() {
            self.produce_token("scope".to_owned(), length, true);
        }
        self.tokens
    }
}

