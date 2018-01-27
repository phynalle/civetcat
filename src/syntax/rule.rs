use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use syntax::regex::{self, Regex};
use syntax::str_piece::StrPiece;
use syntax::raw_rule::{RawRule, RawCapture};
use syntax::loader::Loader;
use lazy::Lazy;

pub type RuleId = usize;

pub struct MatchResult {
    pub rule: RuleId,
    pub caps: regex::MatchResult,
}

pub enum Type {
    Include,
    Match,
    BeginEnd,
    BeginWhile,
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
            Inner::BeginWhile(ref r) => r.id,
        }
    }

    pub fn name(&self) -> Option<String> {
        match **self.inner {
            Inner::Include(ref r) => r.name.clone(),
            Inner::Match(ref r) => r.name.clone(),
            Inner::BeginEnd(ref r) => r.name.clone(),
            Inner::BeginWhile(ref r) => r.name.clone(),
        }
    }

    pub fn display(&self) -> Type {
        match **self.inner {
            Inner::Include(_) => Type::Include,
            Inner::Match(_) => Type::Match,
            Inner::BeginEnd(_) => Type::BeginEnd,
            Inner::BeginWhile(_) => Type::BeginWhile,
        }
    }

    pub fn downgrade(&self) -> WeakRule {
        WeakRule { inner: Rc::downgrade(&self.inner) }
    }

    fn find_match<'a>(&self, text: StrPiece<'a>, match_results: &mut Vec<MatchResult>) {
        match **self.inner {
            Inner::Include(ref r) => {
                let pats = r.patterns.borrow();
                for pat in &(*pats) {
                    let pat = pat.upgrade().unwrap();
                    pat.find_match(text, match_results);
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
            Inner::BeginWhile(ref r) => {
                let m = r.begin_expr.find(text);
                if m.is_some() {
                    match_results.push(MatchResult {
                        rule: r.id,
                        caps: m.unwrap(),
                    });
                }
            }
        }
    }

    fn match_pattern<'a>(&self, text: StrPiece<'a>) -> Vec<MatchResult> {
        let mut match_results = Vec::new();
        self.find_match(text, &mut match_results);
        match_results
    }

    fn match_subpatterns<'a>(
        &self,
        patterns: &Vec<WeakRule>,
        text: StrPiece<'a>,
    ) -> Vec<MatchResult> {
        let mut results = Vec::new();
        patterns
            .iter()
            .map(|x| x.upgrade().unwrap())
            .for_each(|rule| rule.find_match(text, &mut results));
        results
    }

    pub fn collect_matches<'a>(&self, text: StrPiece<'a>) -> Vec<MatchResult> {
        match **self.inner {
            Inner::Include(_) => self.match_pattern(text),
            Inner::Match(_) => Vec::new(),
            Inner::BeginEnd(ref r) => self.match_subpatterns(&r.patterns, text),
            Inner::BeginWhile(ref r) => self.match_subpatterns(&r.patterns, text),
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

    pub fn do_beginwhile<F: FnOnce(&BeginWhileRule)>(&self, func: F) {
        if let Inner::BeginWhile(ref rule) = **self.inner {
            func(rule)
        }
    }

    pub fn has_match(&self) -> bool {
        match **self.inner {
            Inner::Include(ref r) => !r.patterns.borrow().is_empty(),
            _ => true,
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
    BeginWhile(BeginWhileRule),
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
    pub content_name: Option<String>,

    pub begin_expr: Regex,
    pub end_expr: String,
    pub begin_captures: CaptureGroup,
    pub end_captures: CaptureGroup,

    pub patterns: Vec<WeakRule>,
}

pub struct BeginWhileRule {
    pub id: RuleId,
    pub name: Option<String>,

    pub begin_expr: Regex,
    pub while_expr: String,
    pub begin_captures: CaptureGroup,
    pub patterns: Vec<WeakRule>,
}

pub struct CaptureGroup(pub HashMap<usize, WeakRule>);

// ! Using RefWrapper reduces safy of the program because it has a possibility that violates
// the rules on lifetimes of references
struct RefWrapper<T>(*const T);

impl<T> Clone for RefWrapper<T> {
    fn clone(&self) -> RefWrapper<T> {
        RefWrapper(self.0)
    }
}

impl<T> Copy for RefWrapper<T> {}

impl<T> Deref for RefWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> RefWrapper<T> {
    fn new(reference: &T) -> RefWrapper<T> {
        RefWrapper(reference as *const T)
    }
}

pub struct Grammar {
    root_id: RuleId,
    rules: Vec<Rule>,
}

impl Grammar {
    pub fn rule(&self, id: RuleId) -> Rule {
        self.rules[id].clone()
    }

    pub fn root_id(&self) -> RuleId {
        self.root_id
    }
}

pub struct GrammarBuilder {
    next_id: RuleId,
    loader: Box<Loader>,
    sources: HashMap<String, RawRule>,
    rules: HashMap<usize, Rule>,
    src_rule: RawRule,
}

impl GrammarBuilder {
    pub fn new(rule: RawRule, loader: Box<Loader>) -> GrammarBuilder {
        GrammarBuilder {
            next_id: 0,
            loader: loader,
            sources: HashMap::new(),
            rules: HashMap::new(),
            src_rule: rule,
        }
    }

    fn get_source(&mut self, source: &str) -> RefWrapper<RawRule> {
        if let Some(rule_ref) = self.sources.get(source) {
            return RefWrapper::new(rule_ref);
        }


        let rule = self.loader.load(source).unwrap();
        let rule_ref = self.sources.entry(source.to_owned()).or_insert(rule);
        RefWrapper::new(rule_ref)
    }

    pub fn build(&mut self) -> Grammar {
        let root = {
            let raw = RefWrapper::new(&self.src_rule);
            let ctx = Context::new(raw, raw);
            self.compile_rule(&*raw, &ctx)
        };

        let mut rules = Vec::new();
        for i in 0..self.next_id {
            rules.push(self.rules[&i].clone());
        }

        Grammar {
            rules,
            root_id: root.id(),
        }
    }

    fn create_rule(&mut self, rule_id: RuleId, rule: &RawRule, ctx: &Context) -> Inner {
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

            let patterns = if rule.repository.is_some() {
                let mut ctx = ctx.clone();
                ctx.st.push(RefWrapper::new(rule));
                self.compile_patterns(&rule.patterns, &ctx)
            } else {
                self.compile_patterns(&rule.patterns, ctx)
            };

            Inner::Include(IncludeRule {
                id: rule_id,
                name: name,
                patterns: RefCell::new(patterns),
            })
        } else if rule.while_expr.is_some() {
            Inner::BeginWhile(BeginWhileRule {
                id: rule_id,
                name: rule.name.clone(),
                begin_expr: Regex::new(rule.begin.as_ref().unwrap()),
                while_expr: rule.while_expr.clone().unwrap(),
                begin_captures: self.compile_captures(&rule.begin_captures, ctx),
                patterns: self.compile_patterns(&rule.patterns, ctx),
            })
        } else {
            Inner::BeginEnd(BeginEndRule {
                id: rule_id,
                name: rule.name.clone(),
                content_name: rule.content_name.clone(),
                begin_expr: Regex::new(rule.begin.as_ref().unwrap()),
                end_expr: rule.end.clone().unwrap(),
                begin_captures: self.compile_captures(&rule.begin_captures, ctx),
                end_captures: self.compile_captures(&rule.end_captures, ctx),
                patterns: self.compile_patterns(&rule.patterns, ctx),
            })
        }
    }
    fn compile_rule(&mut self, raw: &RawRule, ctx: &Context) -> Rule {
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

    fn compile_patterns(
        &mut self,
        patterns: &Option<Vec<RawRule>>,
        ctx: &Context,
    ) -> Vec<WeakRule> {
        let mut compiled_patterns = Vec::new();
        if let Some(ref patterns) = *patterns {
            for pattern in patterns {
                let rule = match pattern.include {
                    None => self.compile_rule(pattern, ctx),
                    Some(ref inc) if inc.starts_with('#') => {
                        self.compile_rule(ctx.search_pattern(&inc[1..]), ctx)
                    }

                    Some(ref inc) if inc == "$base" => {
                        self.rules[ctx.base.id.get().as_ref().unwrap()].clone()
                    }
                    Some(ref inc) if inc == "$self" => {
                        self.rules[ctx._self.id.get().as_ref().unwrap()].clone()
                    }
                    Some(ref inc) if inc.contains('#') => {
                        let external_sources: Vec<_> = inc.splitn(2, '#').collect();
                        let source = external_sources[0];
                        let pat = external_sources[1];
                        let new_root = self.get_source(source);
                        let ctx = Context::new(ctx._self, new_root);
                        self.compile_rule(ctx.search_pattern(pat), &ctx)
                    }
                    Some(ref inc) => {
                        let new_root = self.get_source(inc);
                        let ctx = Context::new(ctx._self, new_root);
                        self.compile_rule(&*ctx._self, &ctx)
                    }
                };
                compiled_patterns.push(rule.downgrade());
            }
        }
        compiled_patterns
    }

    fn compile_captures(&mut self, captures: &Option<RawCapture>, ctx: &Context) -> CaptureGroup {
        let mut h = HashMap::new();
        if let Some(ref captures) = *captures {
            match *captures {
                RawCapture::Map(ref map) => {
                    for (k, v) in map {
                        let r = self.compile_rule(v, ctx).downgrade();
                        let n = k.parse::<usize>().unwrap();
                        h.insert(n, r);
                    }
                }
                RawCapture::List(ref list) => {
                    let r = self.compile_rule(&list[0], ctx).downgrade();
                    h.insert(0, r);
                }
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

#[derive(Clone)]
struct Context {
    _self: RefWrapper<RawRule>,
    base: RefWrapper<RawRule>,
    st: Vec<RefWrapper<RawRule>>,
}

impl Context {
    fn new(base: RefWrapper<RawRule>, _self: RefWrapper<RawRule>) -> Context {
        Context {
            base: base,
            _self: _self,
            st: vec![_self],
        }
    }

    fn search_pattern(&self, pat: &str) -> &RawRule {
        for rule in &self.st {
            let repo = rule.repository.as_ref().expect(
                "broken format: repository not found",
            );

            if let Some(found) = repo.get(pat) {
                return found;
            }
        }
        panic!("pattern \"{}\" not found in the repository", pat);
    }
}
