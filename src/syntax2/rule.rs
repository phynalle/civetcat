use std::collections::HashMap;
use std::cell::Cell;
use std::rc::Rc;

use syntax2::grammar::Grammar;
use syntax2::regex_set::RegexSet;

pub type RuleId = usize;

#[derive(Debug)]
pub enum Rule {
    Include(IncludeRule),
    Match(MatchRule),
    BeginEnd(BeginEndRule),
//    Capture(CaptureRule),
}

impl Rule {
    #[allow(dead_code)]
    pub fn id(&self) -> RuleId {
        match *self {
            Rule::Include(ref r) => r.id,
            Rule::Match(ref r) => r.id,
            Rule::BeginEnd(ref r) => r.id,
        }
    }

    pub fn find(&self, text: &str, rules: &[Rc<Rule>]) -> Vec<FindResult> {
        let mut res = Vec::new();
        match *self {
            Rule::Include(ref r) => {
                for id in &r.patterns {
                    let x = rules[*id].find(text, rules);
                    res.extend(x);
                }
            }
            Rule::Match(ref r) => {
                let mut m = r.match_src.find(text);
                if !m.is_empty() {
                    let mr = m.remove(0);
                    let r = FindResult {
                        id: r.id,
                        start: mr.start,
                        end: mr.end,
                        groups: mr.groups,
                    };
                    res.push(r);
                }
            }
            Rule::BeginEnd(ref r) => {
                let mut m = r.begin.find(text);
                if !m.is_empty() {
                    let mr = m.remove(0);
                    let r = FindResult {
                        id: r.id,
                        start: mr.start,
                        end: mr.end,
                        groups: mr.groups,
                    };
                    res.push(r);
                }
            }
        }
        res
    }

    pub fn find_subpattern(&self, text: &str, rules: &[Rc<Rule>]) -> Vec<FindResult> {
        let mut res = Vec::new();
        match *self {
            Rule::Include(_) => {
                let x = self.find(text, rules);
                res.extend(x);
            }
            Rule::Match(_) => {
                panic!("Impossible");
            }
            Rule::BeginEnd(ref r) => {
                for id in &r.patterns {
                    let x = rules[*id].find(text, rules);
                    res.extend(x);
                }
            }
        }
        res
    }

    pub fn capture_group(&self) -> Option<&CaptureGroup> {
        match *self {
            Rule::Include(_) => None,
            Rule::Match(ref rule) => Some(&rule.captures),
            Rule::BeginEnd(ref rule) => Some(&rule.begin_captures),
        }
    }
}

pub struct FindResult {
    pub id: RuleId,
    pub start: usize,
    pub end: usize,
    pub groups: Vec<(usize, usize)>,
}

#[derive(Debug)]
pub struct IncludeRule {
    pub id: RuleId,
    patterns: Vec<RuleId>,
}

#[derive(Debug)]
pub struct MatchRule {
    pub id: RuleId,
    pub name: Option<String>,
    pub match_src: RegexSet,
    pub captures: CaptureGroup,
}

#[derive(Debug)]
pub struct BeginEndRule {
    pub id: RuleId,
    pub name: Option<String>,
    pub begin: RegexSet,
    pub end: String,
    pub begin_captures: CaptureGroup,
    pub end_captures: CaptureGroup,
    patterns: Vec<RuleId>,
}

#[derive(Debug, Clone)]
pub struct CaptureRule {
    pub name: Option<String>,
    pub rule_id: Option<RuleId>,
}

#[derive(Debug)]
pub struct CaptureGroup(pub HashMap<usize, CaptureRule>);

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

    pub fn compile(mut self, rule: &mut RawRule) -> Grammar {
        let repo = HashMap::new();
        let root_id = self.compile_rule(rule, rule.repository.as_ref().unwrap_or(&repo));

        let max_rule_id = *self.rules.keys().max().unwrap();
        assert_eq!(max_rule_id + 1, self.rules.len());

        let mut rules = Vec::with_capacity(max_rule_id + 1);
        let mut ruless = &mut self.rules;
        for i in 0..(max_rule_id + 1) {
            rules.push(Rc::new(ruless.remove(&i).unwrap()));
        }

        Grammar::new(rules, root_id)
    }

    fn compile_rule(&mut self, rule: &RawRule, repo: &HashMap<String, RawRule>) -> RuleId {
        if let Some(rule_id) = rule.id.get() {
            return rule_id;
        }

        let rule_id = self.next_rule();
        rule.id.set(Some(rule_id));

        let compiled_rule = self._create_rule(rule_id, rule, repo);
        self.rules.insert(rule_id, compiled_rule);
        rule_id
    }

    fn _create_rule(&mut self,
                    id: RuleId,
                    rule: &RawRule,
                    repo: &HashMap<String, RawRule>)
                    -> Rule {
        if rule.match_expr.is_some() {
            let match_rule = MatchRule {
                id,
                name: rule.name.clone(),
                match_src: RegexSet::with_patterns(&[rule.match_expr.as_ref().unwrap()]),
                captures: self.compile_captures(&rule.captures, repo),
            };
            return Rule::Match(match_rule);
        }

        if rule.begin.is_none() {
            let include_rule = IncludeRule {
                id,
                patterns: self.compile_patterns(&rule.patterns, repo),
            };
            return Rule::Include(include_rule);
        }

        let begin_end_rule = BeginEndRule {
            id,
            name: rule.name.clone(),
            begin: RegexSet::with_patterns(&[rule.begin.as_ref().unwrap()]),
            begin_captures: self.compile_captures(&rule.begin_captures, repo),
            end: rule.end.clone().unwrap(),
            end_captures: self.compile_captures(&rule.end_captures, repo),
            patterns: self.compile_patterns(&rule.patterns, repo),
        };
        Rule::BeginEnd(begin_end_rule)
    }

    fn compile_patterns(&mut self,
                        patterns: &Option<Vec<RawRule>>,
                        repo: &HashMap<String, RawRule>)
                        -> Vec<RuleId> {
        let mut rules = Vec::new();
        if let Some(ref patterns) = *patterns {
            for pattern in patterns {
                let rule_id = match pattern.include {
                    None => self.compile_rule(pattern, repo),
                    Some(ref inc) =>
                        if inc.starts_with('#') {
                            match repo.get(&inc[1..]) {
                                Some(rule) => self.compile_rule(rule, repo),
                                None => panic!("not found"),
                            }
                        } else if inc == "$base" || inc == "$self" {
                            0
                        } else {
                            panic!("unimplemented yet...");
                        }
                };
                rules.push(rule_id);
            }
        }
        rules
    }

    fn compile_captures(&mut self,
                        captures: &Option<HashMap<usize, RawRule>>,
                        repo: &HashMap<String, RawRule>)
                        -> CaptureGroup {
        let mut h = HashMap::new();
        if let Some(ref captures) = *captures {
            for (k, v) in captures {
                let rule_id = v.patterns.as_ref().map(|_| self.compile_rule(v, repo));
                let capture_rule = CaptureRule {
                    name: v.name.clone(),
                    rule_id,
                };
                h.insert(*k, capture_rule);
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
