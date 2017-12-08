use std::rc::Rc;
use style::{Style, StyleTree};
use syntax::grammar::{Tokenizer, Grammar};

pub struct LineColorizer {
    scopes: StyleTree,
    grammar: Rc<Grammar>,
}

impl LineColorizer {
    pub fn new(scopes: StyleTree, grammar: Rc<Grammar>) -> LineColorizer {
        LineColorizer { scopes, grammar }
    }

    pub fn process_line(&mut self, line: &str) -> String {
        let mut tokenizer = Tokenizer::new(&*self.grammar);
        let tokens = tokenizer.tokenize_line(line);
        let mut colored_tokens: Vec<_> = tokens
            .into_iter()
            .map(|t| {
                let style = self.scopes.style(&t.scopes);
                format!("{}{}", style.color(), &line[t.start..t.end])
            })
            .collect();
        colored_tokens.push(Style::reset());
        colored_tokens.join("")
    }
}
