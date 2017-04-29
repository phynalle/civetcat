use std::io::Result;
use std::cell::RefCell;
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
    settings: Settings,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    foreground: Option<String>,
    background: Option<String>,
    font_style: Option<String>,
}

impl Settings {
    fn empty() -> Settings {
        Settings {
            foreground: None,
            background: None,
            font_style: None,
        }
    }
}

struct Tree {
    root: Node,
}

impl Tree {
    fn new() -> Tree {
        Tree { root: Node::new(Settings::empty()) }
    }

    fn insert(&mut self, key: &str, value: Settings) {
        let keys: Vec<_> = key.split(".").collect();
        self.root.insert(&keys, value);
    }

    fn print_debug(&self) {
        println!("root");
        self.root.print_debug(1);
    }
}

struct Node {
    // name: String,
    value: Settings,
    children: HashMap<String, RefCell<Node>>,
}

impl Node {
    fn new(value: Settings) -> Node {
        Node {
            value: value,
            children: HashMap::new(),
        }
    }

    fn insert(&mut self, keys: &[&str], value: Settings) {
        assert!(!keys.is_empty());
        if keys.len() == 1 {
            if let Some(node) = self.children.get(keys[0]) {
                node.borrow_mut().value = value;
                return;
            }
            self.children.insert(keys[0].to_string(), RefCell::new(Node::new(value)));

        } else {
            let node = self.children
                .entry(keys[0].to_string())
                .or_insert(RefCell::new(Node::new(Settings::empty())));
            (*node).borrow_mut().insert(&keys[1..], value);
        }
    }

    fn print_debug(&self, depth: usize) {
        use std::iter::repeat;
        let blank: String = repeat("..".to_string()).take(depth).collect();
        for (key, node) in &self.children {
            println!("{}{} -> {:?}", blank, key, node.borrow().value.foreground);
            node.borrow().print_debug(depth + 1);
        }
    }
}

pub fn theme_test() -> Result<()> {
    let f = File::open("themes/Kimbie_dark.json")?;
    let theme: Theme = serde_json::from_reader(f)?;

    let mut tree = Tree::new();
    for scope in &theme.settings {
        if scope.scope.is_none() {
            continue;
        }

        let scope_names: Vec<_> = scope.scope
            .as_ref()
            .unwrap()
            .as_str()
            .split(",")
            .map(|s| s.trim())
            .collect();

        for name in scope_names {
            tree.insert(&name, scope.settings.clone());
        }
    }
    tree.print_debug();

    Ok(())
}
