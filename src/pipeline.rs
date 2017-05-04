use std::io::Cursor;
use std::default::Default;

use colorizer::ScopeTree;
use colorizer::TextColorizer;
use syntax::Syntax;
use tokenizer::{Builder, Grammar};

pub struct Pipeline {
    scopes: ScopeTree,
    builder: Builder,
}

impl Pipeline {
    pub fn new(scopes: ScopeTree, builder: Builder) -> Pipeline {
        Pipeline {
            scopes: scopes,
            builder: builder,
        }
    }

    pub fn process(&mut self, text: &str) -> Cursor<String> {
        let mut tok = self.builder.build();
        let colored = text.lines()
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
            .collect::<String>();
        Cursor::new(colored)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        let scopes = ScopeTree::create("themes/Kimbie_dark.json").unwrap();
        let grammar = load_grammar("syntaxes/rust.tmLanguage.json");
        let builder = Builder::new(grammar);
        Pipeline::new(scopes, builder)
    }
}

fn load_grammar(filename: &str) -> Grammar {
    match Syntax::new(filename) {
        Ok(s) => s.compact(),
        _ => panic!("panic~~"),
    }
}
