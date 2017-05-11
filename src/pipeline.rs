use std::rc::Rc;

use colorizer::ScopeTree;
use colorizer::TextColorizer;
use tokenizer::{Tokenizer, Grammar};

pub struct Pipeline {
    scopes: ScopeTree,
    grammar: Rc<Grammar>,
}

impl Pipeline {
    pub fn new(grammar: Rc<Grammar>) -> Pipeline {
        Pipeline {
            scopes: ScopeTree::create("themes/Kimbie_dark.json").unwrap(),
            grammar,
        }
    }

    #[allow(dead_code)]
    pub fn process(&mut self, text: &str) -> String {
        let mut tok = Tokenizer::new(self.grammar.clone());
        text.lines()
            .map(|line| (line, tok.tokenize(line)))
            .map(|(line, tokens)| {
                let mut v: Vec<_> = tokens.into_iter()
                    .filter_map(|t| {
                        let sts = self.scopes.get(&t.2);
                        sts.and_then(|sts| {
                            if !sts.is_empty() {
                                Some((t.0, t.1, sts))
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                v.sort_by(|&(ax, ay, _), &(bx, by, _)| (ax, ay).cmp(&(bx, by)));
                (line, v)
            })
            .map(|(line, tokens)| {
                let mut s = line.to_owned();
                for p in TextColorizer::process(&tokens) {
                    s.insert_str(p.0, &p.1);
                }
                s.push('\n');
                s
            })
            .collect::<String>()
    }

    pub fn process_line(&mut self, line: &str) -> String {
        let mut tok = Tokenizer::new(self.grammar.clone());
        let tokens = tok.tokenize(line);
        let mut tokens: Vec<_> = tokens.into_iter()
            .filter_map(|t| {
                let sts = self.scopes.get(&t.2);
                sts.and_then(|sts| {
                    if !sts.is_empty() {
                        Some((t.0, t.1, sts))
                    } else {
                        None
                    }
                })
            })
            .collect();
            tokens.sort_by(|&(ax, ay, _), &(bx, by, _)| (ax, ay).cmp(&(bx, by)));
            let mut s = line.to_owned();
            for p in TextColorizer::process(&tokens) {
                s.insert_str(p.0, &p.1);
            }
            s
    }
}
