use std::rc::{Rc, Weak};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use syntax::grammar::Grammar;
use syntax::regex::{self, Regex};
use syntax::str_piece::StrPiece;
use lazy::Lazy;
pub type RuleId = usize;

pub struct MatchResult {
    pub rule: RuleId,
    pub caps: regex::MatchResult,
}

#[derive(Clone)]
pub struct Rule {
    inner: Rc<Lazy<Inner>>,
}

impl Rule {
    pub fn new() -> Rule {
        Rule { inner: Rc::new(Lazy::new()) }
    }

    pub fn assign(&self, rule: Inner) {
        self.inner.init(rule);
    }

    pub fn id(&self) -> RuleId {
        match **self.inner {
            Inner::Include(ref r) => r.id,
            Inner::Match(ref r) => r.id,
            Inner::BeginEnd(ref r) => r.id,
        }
    }

    pub fn name(&self) -> Option<String> {
        match **self.inner {
            Inner::Include(ref r) => r.name.clone(),
            Inner::Match(ref r) => r.name.clone(),
            Inner::BeginEnd(ref r) => r.name.clone(),
        }
    }

    pub fn downgrade(&self) -> WeakRule {
        WeakRule { inner: Rc::downgrade(&self.inner) }
    }

    pub fn match_patterns<'a>(&self, text: StrPiece<'a>) -> Vec<MatchResult> {
        let mut match_results = Vec::new();
        match **self.inner {
            Inner::Include(ref r) => {
                let patterns = r.patterns.borrow();
                for pattern in (*patterns).iter().map(|rule| rule.upgrade().unwrap()) {
                    match_results.extend(pattern.match_patterns(text));
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

    pub fn match_subpatterns<'a>(&self, text: StrPiece<'a>) -> Vec<MatchResult> {
        match **self.inner {
            Inner::Include(_) => self.match_patterns(text),
            Inner::Match(_) => Vec::new(),
            Inner::BeginEnd(ref r) => {
                let mut match_results = Vec::new();
                for pattern in r.patterns.iter().map(|x| x.upgrade().unwrap()) {
                    match_results.extend(pattern.match_patterns(text));
                }
                match_results
            }
        }
    }

    pub fn do_include<F: FnOnce(&IncludeRule)>(&self, func: F) {
        if let Inner::Include(ref rule) = **self.inner {
            func(rule)
        }
    }
    pub fn do_match<F: FnOnce(&MatchRule)>(&self, func: F) {
        if let Inner::Match(ref rule) = **self.inner {
            func(rule)
        }
    }
    pub fn do_beginend<F: FnOnce(&BeginEndRule)>(&self, func: F) {
        if let Inner::BeginEnd(ref rule) = **self.inner {
            func(rule)
        }
    }
}

#[derive(Clone)]
pub struct WeakRule {
    inner: Weak<Lazy<Inner>>,
}

impl WeakRule {
    pub fn upgrade(&self) -> Option<Rule> {
        self.inner.upgrade().map(|rc| Rule { inner: rc })
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
    patterns: RefCell<Vec<WeakRule>>,
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

    pub patterns: Vec<WeakRule>,
}


pub struct CaptureGroup(pub HashMap<usize, WeakRule>);

pub struct Compiler {
    rules: HashMap<usize, Rule>,
    next_id: RuleId,
}

#[derive(Clone, Copy)]
struct Context<'a> {
    _self: &'a RawRule,
    base: &'a RawRule,
}

impl Compiler {
    pub fn new(src: &str) -> Compiler {
        Compiler {
            rules: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn compile(&mut self, raw: &RawRule) -> Grammar {
        let ctx = Context {
            _self: raw,
            base: raw,
        };

        let root = self.compile_rule(raw, ctx);
        let mut rules = Vec::new();
        for i in 0..self.next_id {
            rules.push(self.rules[&i].clone());
        }

        Grammar::new(rules, root.id())
    }

    fn create_rule<'a>(&mut self, rule_id: RuleId, rule: &RawRule, ctx: Context<'a>) -> Inner {
        if rule.match_expr.is_some() {
            Inner::Match(MatchRule {
                id: rule_id,
                name: rule.name.clone(),
                expr: Regex::new(rule.match_expr.as_ref().unwrap()),
                captures: self.compile_captures(&rule.captures, ctx),
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
                patterns: RefCell::new(self.compile_patterns(&rule.patterns, ctx)),
            })
        } else {
            Inner::BeginEnd(BeginEndRule {
                id: rule_id,
                name: rule.name.clone(),
                begin_expr: Regex::new(rule.begin.as_ref().unwrap()),
                end_expr: rule.end.clone().unwrap(),
                begin_captures: self.compile_captures(&rule.begin_captures, ctx),
                end_captures: self.compile_captures(&rule.end_captures, ctx),
                patterns: self.compile_patterns(&rule.patterns, ctx),
            })
        }
    }

    fn compile_rule<'a>(&mut self, raw: &RawRule, ctx: Context<'a>) -> Rule {
        match raw.id.get() {
            Some(rule_id) => self.rules[&rule_id].clone(),
            None => {
                let rule_id = self.next_rule_id();
                raw.id.set(Some(rule_id));

                let rule = Rule::new();
                self.rules.insert(rule_id, rule.clone());
                rule.assign(self.create_rule(rule_id, raw, ctx));
                rule
            }
        }
    }

    fn compile_patterns<'a>(
        &mut self,
        patterns: &Option<Vec<RawRule>>,
        ctx: Context<'a>,
    ) -> Vec<WeakRule> {
        let mut compiled_patterns = Vec::new();
        if let Some(ref patterns) = *patterns {
            for pattern in patterns {
                let rule = match pattern.include {
                    None => self.compile_rule(pattern, ctx),
                    Some(ref inc) if inc.starts_with('#') => {
                        let repo = ctx._self.repository.as_ref().expect(
                            "broken format: repository not found",
                        );
                        match repo.get(&inc[1..]) {
                            Some(rule) => self.compile_rule(rule, ctx),
                            None => panic!("pattern {} not Found in the repository", inc),
                        }
                    }
                    Some(ref inc) if inc == "$base" => {
                        self.rules[ctx.base.id.get().as_ref().unwrap()].clone()
                    }
                    Some(ref inc) if inc == "$self" => {
                        self.rules[ctx._self.id.get().as_ref().unwrap()].clone()
                    }
                    Some(ref inc) => panic!("Unexpected pattern {}", inc),
                };
                compiled_patterns.push(rule.downgrade());
            }
        }
        compiled_patterns
    }

    fn compile_captures<'a>(
        &mut self,
        captures: &Option<HashMap<usize, RawRule>>,
        ctx: Context<'a>,
    ) -> CaptureGroup {
        let mut h = HashMap::new();
        if let Some(ref captures) = *captures {
            for (k, v) in captures {
                h.insert(*k, self.compile_rule(v, ctx).downgrade());
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
