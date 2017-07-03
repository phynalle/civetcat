use std::io::Result;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

use serde_json;

use tokenizer;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Pattern {
    Include(Include),
    Match(Match),
    Patterns(Patterns),
    Block(Block),
}

#[derive(Deserialize, Debug, Clone)]
struct Patterns {
    pub patterns: Vec<Pattern>,
}

impl Pattern {
    fn compact<'a>(&self, d: &mut Delivery<'a>) -> usize {
        match *self {
            Pattern::Include(ref p) => {
                let path = &p.include[1..];
                if d.cache.contains_key(path) {
                    return d.cache[path];
                }
                if !d.repos.contains_key(path) {
                    panic!("Pattern not found: {}", p.include);
                }
                match *d.repos.get(path).unwrap() {
                    Pattern::Include(_) => panic!("Too deep"),
                    Pattern::Match(ref pp) => Syntax::new_node_from_match2(pp, d, path.to_owned()),
                    Pattern::Block(ref pp) => Syntax::new_node_from_block2(pp, d, path.to_owned()),
                    Pattern::Patterns(ref pp) => {
                        Syntax::new_node_from_patterns2(pp, d, path.to_owned())
                    }
                }
            }
            Pattern::Match(ref p) => Syntax::new_node_from_match(p, d),
            Pattern::Block(ref p) => Syntax::new_node_from_block(p, d),
            Pattern::Patterns(ref p) => Syntax::new_node_from_patterns(p, d),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Include {
    include: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Block {
    #[serde(rename = "name")]
    scope: Option<String>,
    begin: String,
    end: String,
    begin_captures: Option<Captures>,
    end_captures: Option<Captures>,
    patterns: Option<Vec<Pattern>>,
}

#[derive(Deserialize, Debug, Clone)]
struct Match {
    #[serde(rename = "name")]
    scope: Option<String>,
    #[serde(rename = "match")]
    pattern: String,
    captures: Option<Captures>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Syntax {
    name: String,
    scope_name: String,
    file_types: Vec<String>,
    patterns: Vec<Pattern>,
    repository: HashMap<String, Pattern>,
    version: String,
}

impl Syntax {
    pub fn from_text(text: &str) -> Result<Syntax> {
        Ok(serde_json::from_str(text).unwrap())
    }

    fn new_node_from_match<'a>(p: &Match, d: &mut Delivery<'a>) -> tokenizer::ScopeId {
        let m = tokenizer::Match {
            name: p.scope.clone(),
            pat: tokenizer::Pattern {
                pattern: p.pattern.clone(),
                captures: p.captures
                    .as_ref()
                    .map(|caps| match *caps {
                             Captures::Map(ref m) => {
                                 m.iter()
                                     .map(|(key, val)| (key.to_string(), val.name.clone()))
                                     .collect()
                             }
                             Captures::List(ref v) => {
                                 assert!(!v.is_empty());
                                 let mut h = HashMap::new();
                                 h.insert("0".to_owned(), v[0].name.clone());
                                 h
                             }
                         })
                    .unwrap_or_default(),
            },
        };
        let m = tokenizer::Scope::Match(Rc::new(RefCell::new(m)));
        let id = d.nodes.len();
        d.nodes.push(m);
        id
    }

    fn new_node_from_block<'a>(p: &Block, d: &mut Delivery<'a>) -> tokenizer::ScopeId {
        let b = tokenizer::Block {
            name: p.scope.clone(),
            begin: tokenizer::Pattern {
                pattern: p.begin.clone(),
                captures: p.begin_captures
                    .as_ref()
                    .map(|caps| match *caps {
                             Captures::Map(ref m) => {
                                 m.iter()
                                     .map(|(key, val)| (key.to_string(), val.name.clone()))
                                     .collect()
                             }
                             Captures::List(ref v) => {
                                 assert!(!v.is_empty());
                                 let mut h = HashMap::new();
                                 h.insert("0".to_owned(), v[0].name.clone());
                                 h
                             }
                         })
                    .unwrap_or_default(),
            },
            end: tokenizer::Pattern {
                pattern: p.end.clone(),
                captures: p.end_captures
                    .as_ref()
                    .map(|caps| match *caps {
                             Captures::Map(ref m) => {
                                 m.iter()
                                     .map(|(key, val)| (key.to_string(), val.name.clone()))
                                     .collect()
                             }
                             Captures::List(ref v) => {
                                 assert!(!v.is_empty());
                                 let mut h = HashMap::new();
                                 h.insert("0".to_owned(), v[0].name.clone());
                                 h
                             }
                         })
                    .unwrap_or_default(),
            },
            subscopes: Vec::new(),
        };
        let b = tokenizer::Scope::Block(Rc::new(RefCell::new(b)));
        let id = d.nodes.len();
        d.nodes.push(b);
        id
    }

    fn new_node_from_patterns<'a>(p: &Patterns, d: &mut Delivery<'a>) -> tokenizer::ScopeId {
        let b = tokenizer::Patterns { subscopes: Vec::new() };
        let b = tokenizer::Scope::Patterns(Rc::new(RefCell::new(b)));
        let id = d.nodes.len();
        d.nodes.push(b);
        id
    }

    fn new_node_from_match2<'a>(p: &Match,
                                d: &mut Delivery<'a>,
                                path: String)
                                -> tokenizer::ScopeId {
        let id = Syntax::new_node_from_match(p, d);
        d.cache.insert(path, id);
        id
    }

    fn new_node_from_block2<'a>(p: &Block,
                                d: &mut Delivery<'a>,
                                path: String)
                                -> tokenizer::ScopeId {
        let id = Syntax::new_node_from_block(p, d);
        d.cache.insert(path, id);
        let v = p.patterns
            .as_ref()
            .map(|pats| pats.iter().map(|pat| pat.compact(d)).collect())
            .unwrap_or_default();
        if let tokenizer::Scope::Block(ref blk) = d.nodes[id] {
            blk.borrow_mut().subscopes = v;
        }
        id
    }

    fn new_node_from_patterns2<'a>(p: &Patterns,
                                   d: &mut Delivery<'a>,
                                   path: String)
                                   -> tokenizer::ScopeId {
        let id = Syntax::new_node_from_patterns(p, d);
        d.cache.insert(path, id);
        let v = p.patterns.iter().map(|pat| pat.compact(d)).collect();
        if let tokenizer::Scope::Patterns(ref ptrns) = d.nodes[id] {
            ptrns.borrow_mut().subscopes = v;
        }
        id
    }

    pub fn compact(&self) -> tokenizer::Grammar {
        let mut d = Delivery {
            nodes: Vec::new(),
            cache: HashMap::new(),
            repos: &self.repository,
        };

        let scopes = self.patterns
            .iter()
            .map(|p| p.compact(&mut d))
            .collect::<Vec<_>>();

        tokenizer::Grammar {
            repository: HashMap::new().into(),
            scopes: d.nodes,
            global: Rc::new(RefCell::new(tokenizer::Block {
                                             name: None,
                                             begin: tokenizer::Pattern::empty(),
                                             end: tokenizer::Pattern::empty(),
                                             subscopes: scopes,
                                         })),
        }

    }
}

#[derive(Deserialize, Debug, Clone)]
struct Capture {
    name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Captures {
    Map(HashMap<String, Capture>),
    List(Vec<Capture>),
}

#[derive(Clone, Debug)]
pub struct Token {
    text: String,
    pub captures: Vec<(usize, usize, String)>,
}

struct Delivery<'a> {
    nodes: Vec<tokenizer::Scope>,
    cache: HashMap<String, usize>,
    repos: &'a HashMap<String, Pattern>,
}
