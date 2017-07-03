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

    pub fn tokenize_line(&self, mut line: &str, mut prev_state: StateCell) -> TokenResult {
        // Match Priority: end > subpatterns
        let mut tokens = Vec::new();
        loop {
            let (new_state, last) = self.tokenize_string(line, prev_state, &mut tokens);
            prev_state = new_state;
            match last {
                Some(e) => {
                    line = &line[e..];
                }
                None => break,
            }
        }

        TokenResult {
            state: prev_state,
            tokens,
        }
    }

    pub fn tokenize_test(&self, text: &str) {
        println!("tokenizing li");
        let mut state = State::new(self.root.clone());
        for line in text.lines() {
            println!("tokenizing line...: {}", line);
            let result = self.tokenize_line(line, state);
            state = result.state;
            for t in &result.tokens {
                println!("[{}:{}] {} at depth {}", t.start, t.end, t.s, state.depth);
            }
        }
    }

    fn tokenize_string(&self,
                       text: &str,
                       state: StateCell,
                       tokens: &mut Vec<Token>)
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

        let _tokenize_rule = move |tokens: &mut Vec<Token>,
                                   fr: FindResult,
                                   prev_state: StateCell| {
            let rule = self.rule(fr.id);

            // capturing
            let matched_text = &text[fr.start..fr.end];
            let mut state =
                self.tokenize_capture(matched_text, &fr, rule.capture_group(), prev_state, tokens);

            state = State::push(state, rule.clone());
            tokens.push(Token {
                            start: fr.start,
                            end: fr.end,
                            s: matched_text.to_owned(),
                        });

            match *rule {
                Rule::Match(_) => {
                    state = State::pop(state.clone());
                }
                Rule::BeginEnd(ref r) => {
                    println!("it's start: {}", matched_text);
                    state.set_end_rule(r);
                }
                _ => (),
            }
            state
        };

        let _tokenize_end_rule =
            |tokens: &mut Vec<Token>, rule: &Rule, m: MatchResult, state: StateCell, text: &str| {
                // capturing
                let temp_state = state.clone();
                let capture_group = temp_state.captures.borrow(); // rule.capture_group();
                let matched_text = &text[m.start..m.end];
                let fr = FindResult {
                    id: rule.id(),
                    start: m.start,
                    end: m.end,
                    groups: m.groups,
                };
                let state =
                    self.tokenize_capture(matched_text, &fr, capture_group.as_ref(), state, tokens);

                println!("it's end: {} at {}", matched_text, state.depth);
                tokens.push(Token {
                                start: m.start,
                                end: m.end,
                                s: matched_text.to_owned(),
                            });

                State::pop(state)
            };

        match (matched_rules, matched_end) {
            (Some(m1), Some(m2)) => {
                if m1.start < m2.start {
                    let last = m1.end;
                    (_tokenize_rule(tokens, m1, state.clone()), Some(last))
                } else {
                    let last = m2.end;
                    (_tokenize_end_rule(tokens, &rule, m2, state.clone(), text), Some(last))
                }
            }
            (Some(m), _) => {
                let last = m.end;
                (_tokenize_rule(tokens, m, state.clone()), Some(last))
            }
            (_, Some(m)) => {
                let last = m.end;
                (_tokenize_end_rule(tokens, &rule, m, state.clone(), text), Some(last))
            }
            _ => (state.clone(), None),
        }
    }

    fn tokenize_capture(&self,
                        text: &str,
                        fr: &FindResult,
                        cg: Option<&CaptureGroup>,
                        mut state: StateCell,
                        tokens: &mut Vec<Token>)
                        -> StateCell {
        let rule = self.rule(fr.id);
        if let Some(capture_group) = cg {
            for (group_number, capture) in &capture_group.0 {
                if let Some(rule_id) = capture.rule_id {
                    let grouped_text = match group_number.clone() {
                        0 => text,
                        n => {
                            let view = fr.groups[n - 1];
                            &text[view.0..view.1]
                        }
                    };
                    println!("Parsed Capture... [{}] {}", rule_id, grouped_text);

                    state = State::push(state, self.rules[rule_id].clone());
                    self.tokenize_string(grouped_text, state.clone(), tokens);
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
    s: String,
}

type StateCell = Rc<State>;

pub struct State {
    rule: Rc<Rule>,
    end_rule: RefCell<Option<RegexSet>>,
    captures: RefCell<Option<CaptureGroup>>,
    parent: Option<StateCell>,
    depth: usize,
}

impl State {
    fn new(rule: Rc<Rule>) -> StateCell {
        let state = State {
            rule,
            end_rule: RefCell::new(None),
            captures: RefCell::new(None),
            parent: None,
            depth: 0,
        };
        Rc::new(state)
    }

    fn push(st: StateCell, rule: Rc<Rule>) -> StateCell {
        let state = State {
            rule,
            end_rule: RefCell::new(None),
            captures: RefCell::new(None),
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
        *self.captures.borrow_mut() = Some(CaptureGroup(rule.end_captures.0.clone()));
    }
}

pub fn load_grammars(path: &str) -> Result<RawRule> {
    let f = File::open(path)?;
    let r: RawRule = serde_json::from_reader(f)?;
    Ok(r)
}
