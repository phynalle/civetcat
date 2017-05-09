use std::io::Result;
use std::cell::RefCell;
use std::rc::Rc;
use std::fs::File;
use std::collections::HashMap;

use serde_json;

use tokenizer;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Pattern {
    Root(Syntax),
    Include(Include),
    Match(Match),
    Block(Block),
}

impl Pattern {
    fn compact(&self) -> tokenizer::Scope {
        match *self {
            Pattern::Include(ref p) => tokenizer::Scope::Include(p.include.clone()),
            Pattern::Block(ref p) => {
                let b = tokenizer::Block {
                    name: p.scope.clone(),
                    begin: tokenizer::Pattern {
                        pattern: p.begin.clone(),
                        captures: p.begin_captures
                            .as_ref()
                            .map(|caps| {
                                caps.iter()
                                    .map(|(key, val)| (key.to_string(), val.name.clone()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                    end: tokenizer::Pattern {
                        pattern: p.end.clone(),
                        captures: p.end_captures
                            .as_ref()
                            .map(|caps| {
                                caps.iter()
                                    .map(|(key, val)| (key.to_string(), val.name.clone()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                    subscopes: p.patterns
                        .as_ref()
                        .map(|pats| {
                            pats.iter()
                                .map(|pat| pat.compact().downgrade())
                                .collect()
                        })
                        .unwrap_or_default(),
                };
                tokenizer::Scope::Block(Rc::new(RefCell::new(b)))

            }
            Pattern::Match(ref p) => {
                let m = tokenizer::Match {
                    name: p.scope.clone(),
                    pat: tokenizer::Pattern {
                        pattern: p.pattern.clone(),
                        captures: p.captures
                            .as_ref()
                            .map(|caps| {
                                caps.iter()
                                    .map(|(key, val)| (key.to_string(), val.name.clone()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    },
                };
                tokenizer::Scope::Match(Rc::new(RefCell::new(m)))
            }
            _ => panic!("unreachable"),
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
    begin_captures: Captures,
    end_captures: Captures,
    patterns: Option<Vec<Pattern>>,
}

#[derive(Deserialize, Debug, Clone)]
struct Match {
    #[serde(rename = "name")]
    scope: Option<String>,
    #[serde(rename = "match")]
    pattern: String,
    captures: Captures,
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
    pub fn new(filename: &str) -> Result<Syntax> {
        let file = File::open(filename)?;
        Ok(serde_json::from_reader(file).unwrap())
    }

    pub fn compact(&self) -> tokenizer::Grammar {
        let repos = self.repository
            .iter()
            .map(|(name, pat)| (name.clone(), pat.compact()))
            .collect::<HashMap<String, tokenizer::Scope>>();
        let unnamed: Vec<_> = self.patterns
            .iter()
            .map(|pat| pat.compact())
            .collect();

        tokenizer::Grammar {
            repository: repos.clone().into(),
            unnamed_repos: unnamed.clone(),
            global: Rc::new(RefCell::new(tokenizer::Block {
                    name: None,
                    begin: tokenizer::Pattern::empty(),
                    end: tokenizer::Pattern::empty(),
                    subscopes: unnamed
                        .iter()
                        .map(|scope| scope.downgrade())
                        .collect(),
                })),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Capture {
    name: String,
}

type Captures = Option<HashMap<String, Capture>>;

#[derive(Clone, Debug)]
pub struct Token {
    text: String,
    pub captures: Vec<(usize, usize, String)>,
}