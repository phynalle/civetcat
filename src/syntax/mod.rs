pub mod raw_rule;
pub mod regex;
pub mod rule;
pub mod str_piece;
pub mod tokenizer;
pub mod loader;

use std::io::Result;
use self::loader::{GlobalSourceLoader, Loader};
pub use self::rule::{Grammar, GrammarBuilder};
pub use self::tokenizer::Tokenizer;

pub fn load_grammar_from_source(src_name: &str) -> Result<Grammar> {
    let loader = Box::new(GlobalSourceLoader);
    let rule = loader
        .load(src_name)
        .expect(&format!("undefined language source: {}", src_name));
    let mut c = GrammarBuilder::new(rule, loader);
    Ok(c.build())
}
