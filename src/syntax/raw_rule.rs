use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::result::Result;

use serde::{Deserialize, Deserializer};
use serde_json;

#[derive(Debug, Clone)]
pub struct RawRuleRef(Rc<RawRule>);

impl RawRuleRef {
    pub fn new(rule: RawRule) -> RawRuleRef {
        RawRuleRef(Rc::new(rule))
    }

    pub fn to_weak(&self) -> WeakRawRuleRef {
        WeakRawRuleRef(Rc::downgrade(&self.0))
    }
}

impl Deref for RawRuleRef {
    type Target = RawRule;

    fn deref(&self) -> &RawRule {
        self.0.as_ref()
    }
}

impl<'de> Deserialize<'de> for RawRuleRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        RawRule::deserialize(deserializer).map(RawRuleRef::new)
    }
}

#[derive(Debug, Clone)]
pub struct WeakRawRuleRef(Weak<RawRule>);

impl WeakRawRuleRef {
    pub fn upgrade(&self) -> Option<RawRuleRef> {
        self.0.upgrade().map(RawRuleRef)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum RawCapture {
    Map(HashMap<String, RawRuleRef>),
    List(Vec<RawRuleRef>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawRule {
    pub id: Cell<Option<usize>>,
    pub include: Option<String>,
    pub name: Option<String>,
    pub scope_name: Option<String>,
    pub content_name: Option<String>,
    #[serde(rename = "match")]
    pub match_expr: Option<String>,
    pub captures: Option<RawCapture>,
    pub begin: Option<String>,
    pub begin_captures: Option<RawCapture>,
    pub end: Option<String>,
    pub end_captures: Option<RawCapture>,
    #[serde(rename = "while")]
    pub while_expr: Option<String>,
    pub patterns: Option<Vec<RawRuleRef>>,
    pub repository: Option<HashMap<String, RawRuleRef>>,
}

impl RawRule {
    pub fn from_str(s: &str) -> serde_json::Result<RawRule> {
        serde_json::from_str(s)
    }
}

#[allow(dead_code)]
pub fn test(s: &str) {
    let _ = RawRule::from_str(s).unwrap();
}
