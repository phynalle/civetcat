use std::rc::Rc;
use style::{Style, StyleTree};
use syntax::rule::Grammar;
use syntax::tokenizer::Tokenizer;

pub struct LineColorizer {
    scopes: StyleTree,
    tokenizer: Tokenizer,
}

impl LineColorizer {
    pub fn new(scopes: StyleTree, grammar: &Rc<Grammar>) -> LineColorizer {
        LineColorizer {
            scopes,
            tokenizer: Tokenizer::new(grammar),
        }
    }

    pub fn process_line(&mut self, line: &str) -> String {
        let tokens = self.tokenizer.tokenize_line(line);
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
