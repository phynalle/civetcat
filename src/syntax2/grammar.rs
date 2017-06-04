use std::io::Result;
use std::collections::HashMap;
use std::fs::File;

use std::rc::Rc;
use std::cell::RefCell;

use serde_json;

use syntax2::rule::{CaptureGroup, MatchRule, RuleId, RawRule};

struct Repository {
    inner: HashMap<String, RuleId>,
}

impl Repository {
    fn new() -> Repository {
        Repository {
            inner: HashMap::new(),
        }
    }

}

pub fn load_grammars(path: &str) -> Result<RawRule> {
    let f = File::open(path)?;
    let r: RawRule = serde_json::from_reader(f)?;
    Ok(r)
}