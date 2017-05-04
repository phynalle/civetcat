use std::io::Result;
use std::collections::HashMap;
use std::fs::File;

use serde_json;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Theme {
    author: String,
    name: String,
    comment: String,
    semantic_class: String,
    color_space_name: String,
    settings: Vec<Scope>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Scope {
    name: Option<String>,
    scope: Option<String>,
    #[serde(rename="settings")]
    style: Style,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Style {
    foreground: Option<usize>,
    background: Option<usize>,
    font_style: Option<String>,
}

impl Style {
    pub fn empty() -> Style {
        Style {
            foreground: None,
            background: None,
            font_style: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.foreground.is_none() && self.background.is_none() && self.font_style.is_none()
    }

    #[allow(dead_code)]
    pub fn from(&self, other: Option<Style>) -> Style {
        let mut new = self.clone();
        if let Some(set) = other {
            if new.foreground.is_none() {
                new.foreground = set.foreground;
            }
            if new.background.is_none() {
                new.background = set.background;
            }
            if new.font_style.is_none() {
                new.font_style = set.font_style;
            }
        }
        new
    }

    pub fn color(&self) -> String {
        if self.is_empty() {
            return Style::reset();
        }

        let mut appended = false;
        let mut s: String = "\x1B[".to_owned();
        if let Some(ref style) = self.font_style {
            appended = match style.to_lowercase().as_ref() {
                "bold" => {
                    s.push('1');
                    true
                }
                "italic" => {
                    s.push('4');
                    true
                }
                _ => false,
            };
        }
        if let Some(fg) = self.foreground {
            if appended {
                s.push(';');
            }
            appended = true;
            s.push_str(&format!("38;5;{}", fg));
        }
        if let Some(bg) = self.background {
            if appended {
                s.push(';');
            }
            s.push_str(&format!("48;5;{}", bg));
        }
        s.push('m');
        s
    }

    pub fn reset() -> String {
        "\x1B[0m".to_owned()
    }
}

pub struct ScopeTree {
    root: Node,
}

impl ScopeTree {
    pub fn new() -> ScopeTree {
        ScopeTree { root: Node::new(Style::empty()) }
    }

    pub fn create(filename: &str) -> Result<ScopeTree> {
        let f = File::open(filename)?;
        let theme: Theme = serde_json::from_reader(f)?;
        let mut tree = ScopeTree::new();
        for scope in &theme.settings {
            if scope.scope.is_none() {
                continue;
            }
            let scope_names: Vec<_> = scope.scope
                .as_ref()
                .unwrap()
                .as_str()
                .split(',')
                .map(|s| s.trim())
                .collect();

            for name in scope_names {
                tree.insert(name, scope.style.clone());
            }
        }
        Ok(tree)
    }

    fn insert(&mut self, key: &str, value: Style) {
        let keys: Vec<_> = key.split('.').collect();
        self.root.insert(&keys, value);
    }

    pub fn get(&self, key: &str) -> Option<Style> {
        let keys: Vec<_> = key.split('.').collect();
        self.root.get(&keys)
    }

    // fn print_debug(&self) {
    //     println!("root");
    //     self.root.print_debug(1);
    // }
}

struct Node {
    // name: String,
    value: Style,
    children: HashMap<String, Node>,
}

impl Node {
    fn new(value: Style) -> Node {
        Node {
            value: value,
            children: HashMap::new(),
        }
    }

    fn insert(&mut self, keys: &[&str], value: Style) {
        assert!(!keys.is_empty());
        if keys.len() == 1 {
            if let Some(node) = self.children.get_mut(keys[0]) {
                node.value = value;
                return;
            }
            self.children.insert(keys[0].to_string(), Node::new(value));
        } else {
            let node = self.children
                .entry(keys[0].to_string())
                .or_insert_with(|| Node::new(Style::empty()));
            (*node).insert(&keys[1..], value);
        }
    }

    fn get(&self, keys: &[&str]) -> Option<Style> {
        if !keys.is_empty() {
            if let Some(node) = self.children.get(keys[0]) {
                let v = node.get(&keys[1..]);
                if v.is_some() && !v.as_ref().unwrap().is_empty() {
                    return v;
                }
            }
        }
        Some(self.value.clone())
    }

    // fn print_debug(&self, depth: usize) {
    //     use std::iter::repeat;
    //     let blank: String = repeat("..".to_string()).take(depth).collect();
    //     for (key, node) in &self.children {
    //         println!("{}{} -> {:?}", blank, key, node.value.foreground);
    //         node.print_debug(depth + 1);
    //     }
    // }
}

pub struct TextColorizer<'a> {
    stack: Vec<&'a (usize, usize, Style)>,
    order: Vec<(usize, String)>,
    offset: usize,
}

impl<'a> TextColorizer<'a> {
    fn new() -> TextColorizer<'a> {
        TextColorizer {
            stack: Vec::new(),
            order: Vec::new(),
            offset: 0,
        }
    }

    pub fn process(tokens: &'a [(usize, usize, Style)]) -> Vec<(usize, String)> {
        let mut tc = TextColorizer::new();
        tc.apply(tokens);
        tc.take()
    }

    fn top(&self) -> Option<&'a (usize, usize, Style)> {
        if self.stack.is_empty() {
            None
        } else {
            Some(self.stack[self.stack.len() - 1])
        }
    }

    fn push(&mut self, p: &'a (usize, usize, Style)) {
        let s = p.2.color();
        let incr = s.len();

        self.stack.push(p);
        self.order.push((p.0 + self.offset, s));
        self.offset += incr;
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn pop_until<F>(&mut self, f: F)
        where F: Fn(&'a (usize, usize, Style)) -> bool
    {
        while !self.is_empty() {
            let top = self.top().unwrap();
            if !f(top) {
                break;
            }
            self.stack.pop();
            let code = if self.is_empty() {
                Style::reset()
            } else {
                self.top().unwrap().2.color()
            };

            let incr = code.len();
            self.order.push((top.1 + self.offset, code));
            self.offset += incr;
        }
    }

    fn apply(&mut self, pairs: &'a [(usize, usize, Style)]) {
        for p in pairs {
            if self.is_empty() {
                self.push(p);
                continue;
            }

            let top = self.top().unwrap();
            if top.0 <= p.0 && p.1 <= top.1 {
                self.push(p);
                continue;
            }
            self.pop_until(|top| top.1 <= p.0);
            self.push(p);
        }
        self.pop_until(|_| true);
    }

    fn take(self) -> Vec<(usize, String)> {
        self.order
    }
}
