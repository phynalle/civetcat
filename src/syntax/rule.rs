use std::rc::Rc;
use std::cell::Cell;
use std::collections::HashMap;
use syntax::grammar::Grammar;
use syntax::regex::{self, Regex};

pub type RuleId = usize;

pub struct MatchResult {
    pub rule: RuleId,
    pub caps: regex::MatchResult,
}

#[derive(Clone)]
pub struct Rule {
    inner: Rc<Inner>,
}

impl Rule {
    pub fn new(rule: Inner) -> Rule {
        Rule { inner: Rc::new(rule) }
    }

    pub fn id(&self) -> RuleId {
        match *self.inner {
            Inner::Include(ref r) => r.id,
            Inner::Match(ref r) => r.id,
            Inner::BeginEnd(ref r) => r.id,
        }
    }

    pub fn name(&self) -> Option<String> {
        match *self.inner {
            Inner::Include(ref r) => r.name.clone(),
            Inner::Match(ref r) => r.name.clone(),
            Inner::BeginEnd(ref r) => r.name.clone(),
        }
    }

    pub fn match_patterns(&self, text: &str, rules: &[Rule]) -> Vec<MatchResult> {
        let mut match_results = Vec::new();
        match *self.inner {
            Inner::Include(ref r) => {
                for pattern in r.patterns.iter().map(|&i| rules[i].clone()) {
                    match_results.extend(pattern.match_patterns(text, rules));
                }
            }
            Inner::Match(ref r) => {
                let m = r.expr.find(text);
                if m.is_some() {
                    match_results.push(MatchResult {
                        rule: r.id,
                        caps: m.unwrap(),
                    });
                }
            }
            Inner::BeginEnd(ref r) => {
                let m = r.begin_expr.find(text);
                if m.is_some() {
                    match_results.push(MatchResult {
                        rule: r.id,
                        caps: m.unwrap(),
                    });
                }
            }
        }
        match_results
    }

    pub fn match_subpatterns(&self, text: &str, rules: &[Rule]) -> Vec<MatchResult> {
        match *self.inner {
            Inner::Include(_) => self.match_patterns(text, rules),
            Inner::Match(_) => Vec::new(),
            Inner::BeginEnd(ref r) => {
                let mut match_results = Vec::new();
                for i in &r.patterns {
                    match_results.extend(rules[*i].match_patterns(text, rules));
                }
                match_results
            }
        }
    }

    pub fn do_include<F: FnOnce(&IncludeRule)>(&self, func: F) {
        if let Inner::Include(ref rule) = *self.inner {
            func(rule)
        }
    }
    pub fn do_match<F: FnOnce(&MatchRule)>(&self, func: F) {
        if let Inner::Match(ref rule) = *self.inner {
            func(rule)
        }
    }
    pub fn do_beginend<F: FnOnce(&BeginEndRule)>(&self, func: F) {
        if let Inner::BeginEnd(ref rule) = *self.inner {
            func(rule)
        }
    }
}

pub enum Inner {
    Include(IncludeRule),
    Match(MatchRule),
    BeginEnd(BeginEndRule),
    // BeginWhile,
}

pub struct IncludeRule {
    pub id: RuleId,
    pub name: Option<String>,
    patterns: Vec<RuleId>,
}

pub struct MatchRule {
    pub id: RuleId,
    pub name: Option<String>,
    pub expr: Regex,
    pub captures: CaptureGroup,
}

pub struct BeginEndRule {
    pub id: RuleId,
    pub name: Option<String>,

    pub begin_expr: Regex,
    pub end_expr: String,
    pub begin_captures: CaptureGroup,
    pub end_captures: CaptureGroup,

    pub patterns: Vec<RuleId>,
}

pub struct CaptureGroup(pub HashMap<usize, usize>);

pub struct Compiler {
    rules: HashMap<usize, Rule>,
    next_id: RuleId,
}

impl Compiler {
    pub fn new() -> Compiler {
        Compiler {
            rules: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn compile(&mut self, raw: &mut RawRule) -> Grammar {
        let repo = HashMap::new();
        let root_id = self.compile_rule(raw, raw.repository.as_ref().unwrap_or(&repo));

        let mut rules = Vec::new();
        for i in 0..self.next_id {
            rules.push(self.rules[&i].clone());
        }

        Grammar::new(rules, root_id)
    }

    fn create_rule(
        &mut self,
        rule_id: RuleId,
        rule: &RawRule,
        repo: &HashMap<String, RawRule>,
    ) -> Rule {
        let inner_rule = if rule.match_expr.is_some() {
            Inner::Match(MatchRule {
                id: rule_id,
                name: rule.name.clone(),
                expr: Regex::new(&rule.match_expr.as_ref().unwrap()),
                captures: self.compile_captures(&rule.captures, repo),
            })
        } else if rule.begin.is_none() {
            let name = if rule.scope_name.is_some() {
                rule.scope_name.clone()
            } else {
                rule.name.clone()
            };

            Inner::Include(IncludeRule {
                id: rule_id,
                name: name,
                patterns: self.compile_patterns(&rule.patterns, repo),
            })
        } else {
            Inner::BeginEnd(BeginEndRule {
                id: rule_id,
                name: rule.name.clone(),
                begin_expr: Regex::new(rule.begin.as_ref().unwrap()),
                end_expr: rule.end.clone().unwrap(),
                begin_captures: self.compile_captures(&rule.begin_captures, repo),
                end_captures: self.compile_captures(&rule.end_captures, repo),
                patterns: self.compile_patterns(&rule.patterns, repo),
            })
        };
        Rule::new(inner_rule)
    }

    fn compile_rule(&mut self, rule: &RawRule, repo: &HashMap<String, RawRule>) -> RuleId {
        match rule.id.get() {
            Some(rule_id) => rule_id,
            None => {
                let rule_id = self.next_rule_id();
                rule.id.set(Some(rule_id));

                let rule = self.create_rule(rule_id, rule, repo);
                self.rules.insert(rule_id, rule);

                rule_id
            }
        }
    }

    fn compile_patterns(
        &mut self,
        patterns: &Option<Vec<RawRule>>,
        repo: &HashMap<String, RawRule>,
    ) -> Vec<RuleId> {
        let mut compiled_patterns = Vec::new();
        if let Some(ref patterns) = *patterns {
            for pattern in patterns {
                let rule_id = match pattern.include {
                    None => self.compile_rule(pattern, repo),
                    Some(ref inc) if inc.starts_with("#") => {
                        match repo.get(&inc[1..]) {
                            Some(rule) => self.compile_rule(rule, repo),
                            None => panic!("pattern {} not Found in the repository", inc),
                        }
                    }
                    Some(ref inc) if inc == "$base" || inc == "$self" => 0,
                    Some(ref inc) => panic!("Unexpected pattern {}", inc),
                };
                compiled_patterns.push(rule_id);
            }
        }
        compiled_patterns
    }

    fn compile_captures(
        &mut self,
        captures: &Option<HashMap<usize, RawRule>>,
        repo: &HashMap<String, RawRule>,
    ) -> CaptureGroup {
        let mut h = HashMap::new();
        if let Some(ref captures) = *captures {
            for (k, v) in captures {
                h.insert(*k, self.compile_rule(v, repo));
            }
        }
        CaptureGroup(h)
    }

    fn next_rule_id(&mut self) -> RuleId {
        let next = self.next_id;
        self.next_id += 1;
        next
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawRule {
    pub id: Cell<Option<usize>>,
    pub include: Option<String>,
    pub name: Option<String>,
    pub scope_name: Option<String>,
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

