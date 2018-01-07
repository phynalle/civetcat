pub mod raw_rule;
pub mod regex;
pub mod rule;
pub mod str_piece;
pub mod tokenizer;

use std::io::Result;
pub use self::rule::{Grammar, GrammarBuilder};
pub use self::tokenizer::Tokenizer;

pub fn load_grammar_from_source(src_name: &str) -> Result<Grammar> {
    let mut c = GrammarBuilder::new(src_name);
    Ok(c.build())
}


