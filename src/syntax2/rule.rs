use std::collections::HashMap;
use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::rc::Rc;

use pcre::Pcre;

pub type RuleId = usize;

#[derive(Debug, Clone)]
pub enum Rule {
    Include(IncludeRule),
    Match(MatchRule),
    BeginEnd(BeginEndRule),
    Capture(CaptureRule),
}

#[derive(Debug, Clone)]
pub struct IncludeRule {
    patterns: Vec<RuleId>,
}

#[derive(Debug, Clone)]
pub struct MatchRule {
    pub name: Option<String>,
    pub match_expr: String,
    pub captures: CaptureGroup,
}

#[derive(Debug, Clone)]
pub struct BeginEndRule {
    pub name: Option<String>,
    pub begin: String,
    pub end: String,
    pub begin_captures: CaptureGroup,
    pub end_captures: CaptureGroup,
    patterns: Vec<RuleId>,
}

#[derive(Debug, Clone)]
struct CaptureRule {
    name: Option<String>,
    rule_id: Option<RuleId>,
}

#[derive(Debug, Clone)]
pub struct CaptureGroup(HashMap<usize, CaptureRule>);

type Patterns = Vec<RuleId>;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawRule {
    #[serde(skip_deserializing)]
    pub id: Cell<Option<usize>>,
    pub include: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "match")]
    pub match_expr: Option<String>,
    pub captures: Option<HashMap<usize, RawRule>>,
    pub begin: Option<String>,
    pub begin_captures: Option<HashMap<usize, RawRule>>,
    pub end: Option<String>,
    pub end_captures: Option<HashMap<usize, RawRule>>,
    pub patterns: Option<Vec<RawRule>>,
    pub repository: Option<HashMap<String, RawRule>>,
}

pub struct Compiler {
    pub rules: HashMap<RuleId, Rule>,
    pub next_rule_id: RuleId,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            rules: HashMap::new(),
            next_rule_id: 0,
        }
    }

    pub fn compile(&mut self, rule: &mut RawRule) {
        let h = HashMap::new();
        self.compile_rule(rule, rule.repository.as_ref().unwrap_or(&h));


        /* let max_rule_id = self.rules.keys().max().unwrap();
        let mut rules = Vec::with_capacity(max_rule_id + 1);
        for i in 0..(max_rule_id + 1) {
            rules.push(self.rules[&i].clone());
        } */
    }

    fn compile_rule(&mut self, rule: &RawRule, repo: &HashMap<String, RawRule>) -> RuleId {
        if let Some(rule_id) = rule.id.get() {
            return rule_id;
        }

        let rule_id = self.next_rule();
        rule.id.set(Some(rule_id));

        let compiled_rule = self._create_rule(rule, repo);
        self.rules.insert(rule_id, compiled_rule);
        rule_id
    }

    fn _create_rule(&mut self, rule: &RawRule, repo: &HashMap<String, RawRule>) -> Rule {
        if rule.match_expr.is_some() {
            let match_rule = MatchRule {
                name: rule.name.clone(),
                match_expr: rule.match_expr.clone().unwrap(),
                captures: self.compile_captures(&rule.captures, repo),
            };
            return Rule::Match(match_rule);
        } 

        if rule.begin.is_none() {
            let include_rule = IncludeRule {
                patterns: self.compile_patterns(&rule.patterns, repo),
            };
            return Rule::Include(include_rule);
        }

        let begin_end_rule = BeginEndRule {
            name: rule.name.clone(),
            begin: rule.begin.clone().unwrap(),
            begin_captures: self.compile_captures(&rule.begin_captures, repo),
            end: rule.end.clone().unwrap(),
            end_captures: self.compile_captures(&rule.end_captures, repo),
            patterns: self.compile_patterns(&rule.patterns, repo),
        };
        Rule::BeginEnd(begin_end_rule)
    }

    fn compile_patterns(&mut self, patterns: &Option<Vec<RawRule>>, repo: &HashMap<String, RawRule>) -> Vec<RuleId> {
        let mut rules = Vec::new();
        if let Some(ref patterns) = *patterns {
            for pattern in patterns {
                let rule_id = match pattern.include {
                    Some(ref inc) => {
                        if inc.starts_with('#') {
                            match repo.get(&inc[1..]) {
                                Some(ref rule) => {
                                    self.compile_rule(rule, repo)
                                }   
                                None => panic!("not found"),
                            }
                        } else if inc == "$base" || inc == "$self" {
                            0
                        } else {
                            panic!("unimplemented yet...");
                        }
                    }
                    None => {
                        self.compile_rule(pattern, repo)
                    }
                };
                rules.push(rule_id);
            };
        }
        rules
    }

    fn compile_captures(&mut self, captures: &Option<HashMap<usize, RawRule>>, repo: &HashMap<String, RawRule>) -> CaptureGroup {
        let mut h = HashMap::new();
        if let Some(ref captures) = *captures {
            for (k, v) in captures {
                let rule_id = v.patterns
                    .as_ref()
                    .map(|_| self.compile_rule(v, repo));
                let capture_rule = CaptureRule {
                    name: v.name.clone(),
                    rule_id,
                };
                h.insert(k.clone(), capture_rule);
            }
        }
        CaptureGroup(h)
    }

    fn next_rule(&mut self) -> RuleId {
        let id = self.next_rule_id;
        self.next_rule_id += 1;
        id
    }
}
