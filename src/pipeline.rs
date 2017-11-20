use std::rc::Rc;

use colorizer::ScopeTree;
use colorizer::TextColorizer;
use syntax::grammar::{Tokenizer, Grammar};

pub struct Pipeline {
    scopes: ScopeTree,
    grammar: Rc<Grammar>,
}

impl Pipeline {
    pub fn new(scopes: ScopeTree, grammar: Rc<Grammar>) -> Pipeline {
        Pipeline { scopes, grammar }
    }

    #[allow(dead_code)]
    pub fn process(&mut self, text: &str) -> String {
        let mut tok = Tokenizer::new(&(*self.grammar));
        text.lines()
            .map(|line| (line, tok.tokenize_line(line)))
            .map(|(line, tokens)| {
                let v: Vec<_> = tokens
                    .into_iter()
                    .map(|t| {
                        let style = self.scopes.style(&t.scopes);
                        (t.start, t.end, style)
                    })
                    .collect();
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
        let mut tok = Tokenizer::new(&*self.grammar);
        let tokens = tok.tokenize_line(line);
        let tokens: Vec<_> = tokens
            .into_iter()
            .map(|t| {
                let style = self.scopes.style(&t.scopes);
                (t.start, t.end, style)
            })
            .collect();
        let mut s = line.to_owned();
        for p in TextColorizer::process(&tokens) {
            s.insert_str(p.0, &p.1);
        }
        s
    }
}
