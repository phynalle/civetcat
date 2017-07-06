use std::io::Result;
use std::fs::File;

use std::borrow::Borrow;
use std::rc::Rc;
use std::cell::RefCell;

use serde_json;

use syntax2::rule::{FindResult, RawRule, Rule, RuleId, BeginEndRule, CaptureGroup};
use syntax2::regex_set::{MatchResult, RegexSet};

/* struct Repository {
    inner: HashMap<String, RuleId>,
}

impl Repository {
    fn new() -> Repository {
        Repository {
            inner: HashMap::new(),
        }
    }

} */

pub struct Grammar {
    rules: Vec<Rc<Rule>>,
    root: Rc<Rule>,
}

impl Grammar {
    pub fn new(rules: Vec<Rc<Rule>>, root_id: RuleId) -> Grammar {
        let root = rules[root_id].clone();
        Grammar { rules, root }
    }

    pub fn tokenize_line(&self, line: &str, mut prev_state: StateCell) -> TokenResult {
        // Match Priority: end > subpatterns
        let mut line_tokens = LineTokens::new(line);
        let (new_state, last) = self.tokenize_string(line, &mut line_tokens, prev_state);

        prev_state = new_state;

        let tokens = line_tokens.get_result();
        TokenResult {
            state: prev_state,
            tokens,
        }
    }

    pub fn tokenize_test(&self, text: &str) {
        let mut state = State::new(self.root.clone());
        for (num, line) in text.lines().enumerate() {
            println!(" * Line[{}]: {}", num, line);
            let result = self.tokenize_line(line, state);
            state = result.state;
            for t in &result.tokens {
                println!("[TOKEN] ({}) [{}:{}] at depth {}", t.scope, t.start, t.end, state.depth);
            }
        }
    }

    fn tokenize_string(&self,
                       text: &str,
                       line_tokens: &mut LineTokens,
                       state: StateCell)
                       -> (StateCell, Option<usize>) {
        let rule: &Rule = state.rule.borrow();
        let matches = rule.find_subpattern(text, &self.rules);
        let matched_rules = matches.into_iter().min_by_key(|m| m.start);
        let matched_end = state
            .end_rule
            .borrow()
            .as_ref()
            .and_then(|ref end_rule| {
                          let mut m = end_rule.find(text);
                          if !m.is_empty() {
                              Some(m.remove(0))
                          } else {
                              None
                          }
                      });

        let _tokenize_rule = move |line_tokens: &mut LineTokens,
                                   fr: FindResult,
                                   prev_state: StateCell| {
            let rule = self.rule(fr.id);

            // capturing
            let mut state =
                self.tokenize_captures(text, &fr, rule.capture_group(), prev_state, line_tokens);

            let matched_text = &text[fr.start..fr.end];
            state = State::push(state, rule.clone());

            let scope_name = match *rule {
                Rule::Match(ref r) => {
                    println!("[Match] {}", matched_text);
                    state = State::pop(state.clone());
                    r.name.clone()
                }
                Rule::BeginEnd(ref r) => {
                    println!("[BeginEnd] BEGIN: {} {}..{}", matched_text, fr.start, fr.end);
                    state.set_end_rule(r);
                    r.name.clone()
                }
                _ => None,
            };
            if let Some(scope) = scope_name {
                /*
                tokens.push(Token {
                                start: fr.start,
                                end: fr.end,
                                scope: scope,
                            });*/
            }
            state
        };

        let _tokenize_end_rule =
            |line_tokens: &mut LineTokens, rule: &Rule, m: MatchResult, state: StateCell, text: &str| {
                // capturing
                let beginend = match *rule {
                    Rule::BeginEnd(ref r) => r,
                    _ => panic!("Unreachable"),
                };

                let capture_group = &beginend.end_captures; 
                let fr = FindResult {
                    id: rule.id(),
                    start: m.start,
                    end: m.end,
                    groups: m.groups,
                };
                let state =
                    self.tokenize_captures(text, &fr, Some(capture_group), state, line_tokens);

                let matched_text = &text[m.start..m.end];
                println!("[BeginEnd] END: {} at {}", matched_text, state.depth);


                if let Some(ref scope) = beginend.name {
                    /*tokens.push(Token {
                                    start: m.start,
                                    end: m.end,
                                    scope: scope.clone(),
                                });*/
                }
                State::pop(state)
            };

        match (matched_rules, matched_end) {
            (Some(m1), Some(m2)) => {
                if m1.start < m2.start {
                    let last = m1.end;
                    (_tokenize_rule(line_tokens, m1, state.clone()), Some(last))
                } else {
                    let last = m2.end;
                    (_tokenize_end_rule(line_tokens, &rule, m2, state.clone(), text), Some(last))
                }
            }
            (Some(m), _) => {
                let last = m.end;
                (_tokenize_rule(line_tokens, m, state.clone()), Some(last))
            }
            (_, Some(m)) => {
                let last = m.end;
                (_tokenize_end_rule(line_tokens, &rule, m, state.clone(), text), Some(last))
            }
            _ => {
                if let Rule::BeginEnd(ref r) = *rule {
                    if let Some(ref scope) = r.name {
                         /*tokens.push(Token {
                                    start: 0,
                                    end: text.len(),
                                    scope: scope.clone(),
                                });
                                */
                    }
                }
                (state.clone(), None)
            }
        }
    }

    fn tokenize_captures(&self,
                        text: &str,
                        fr: &FindResult,
                        cg: Option<&CaptureGroup>,
                        mut state: StateCell,
                        line_tokens: &mut LineTokens)
                        -> StateCell {
        let rule = self.rule(fr.id);
        if let Some(capture_group) = cg {
            for (group_number, capture) in &capture_group.0 {
                let (start, end, grouped_text) = match group_number.clone() {
                    0 => (fr.start, fr.end, &text[fr.start..fr.end]),
                    n => {
                        // println!("[debug] len:{} n:{}", fr.groups.len(), n);
                        match fr.groups[n-1] {
                            Some(view) => (view.0, view.1, &text[view.0..view.1]),
                            None => continue,
                        }
                    }
                };
                let name = match capture.name {
                    Some(ref s) => s.as_str(),
                    None => "",
                };
                println!("[Capture] {} [{}] {}", name, group_number, grouped_text);

                /*tokens.push(Token {
                    start,
                    end,
                    scope: name.to_owned(),
                });*/

                if let Some(rule_id) = capture.rule_id {
                    state = State::push(state, self.rules[rule_id].clone());
                    self.tokenize_string(grouped_text, line_tokens, state.clone());
                    state = State::pop(state);
                }
            }
        }
        state
    }

    fn rule(&self, id: RuleId) -> Rc<Rule> {
        self.rules[id].clone()
    }
}

pub struct TokenResult {
    tokens: Vec<Token>,
    state: StateCell,
}

struct Token {
    start: usize,
    end: usize,
    scope: String,
}

struct LineTokens<'a> {
    line: &'a str,
    last_index: usize,

    tokens: Vec<Token>,
}

impl<'a> LineTokens<'a> {
    fn new(line: &'a str) -> LineTokens {
        LineTokens {
            line,
            last_index: 0,
            tokens: Vec::new(),
        } 
    }

    fn produce(&mut self, scope: String, end_index: usize) {
        self.produce_token(scope, end_index, false)
    }
    
    fn produce_token(&mut self, scope: String, end: usize, from_first: bool) {
        let start = if from_first {
            0
        } else  {
            if self.last_index >= end {
                return; 
            }
            self.last_index
        };
        self.tokens.push(Token {
            start, 
            end,
            scope: scope,
        });
        self.last_index = end;
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

type StateCell = Rc<State>;

pub struct State {
    rule: Rc<Rule>,
    end_rule: RefCell<Option<RegexSet>>,
    parent: Option<StateCell>,
    depth: usize,
}

impl State {
    fn new(rule: Rc<Rule>) -> StateCell {
        let state = State {
            rule,
            end_rule: RefCell::new(None),
            parent: None,
            depth: 0,
        };
        Rc::new(state)
    }

    fn push(st: StateCell, rule: Rc<Rule>) -> StateCell {
        let state = State {
            rule,
            end_rule: RefCell::new(None),
            parent: Some(st.clone()),
            depth: st.depth + 1,
        };
        Rc::new(state)
    }

    fn pop(st: StateCell) -> StateCell {
        assert!(st.parent.is_some());
        st.parent.clone().expect("it should not be top")
    }

    fn set_end_rule(&self, rule: &BeginEndRule) {
        *self.end_rule.borrow_mut() = Some(RegexSet::with_patterns(&[&rule.end]));
    }
}

pub fn load_grammars(path: &str) -> Result<RawRule> {
    let f = File::open(path)?;
    let r: RawRule = serde_json::from_reader(f)?;
    Ok(r)
}


