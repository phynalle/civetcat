use std::io::Result;
use std::collections::HashMap;

use serde_json;

static FONTSTYLE_BOLD: usize = 0x01;
static FONTSTYLE_ITALIC: usize = 0x02;
static FONTSTYLE_UNDERLINE: usize = 0x04;

pub fn load_theme(raw_text: &str) -> Result<StyleTree> {
    StyleTree::create(raw_text)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Theme {
    // author: String,
    // name: String,
    // comment: String,
    // semantic_class: String,
    // color_space_name: String,
    token_colors: Vec<TokenColor>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenColor {
    name: Option<String>,
    scope: Option<JsonScope>,
    #[serde(rename = "settings")]
    style: RawStyle,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum JsonScope {
    S(String),
    L(Vec<String>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawStyle {
    foreground: Option<usize>,
    background: Option<usize>,
    font_style: Option<String>,
}

pub struct StyleTree {
    root: Node,
    default_style: Style,
}

impl StyleTree {
    pub fn new() -> StyleTree {
        StyleTree {
            root: Node::new(Style::empty()),
            default_style: Style::empty(),
        }
    }

    pub fn create(text: &str) -> Result<StyleTree> {
        let theme: Theme = serde_json::from_str(text)?;
        let mut tree = StyleTree::new();
        for token_color in &theme.token_colors {
            if token_color.scope.is_none() {
                // set default style
                let mut style = Style::from(token_color.style.clone());
                style.bg = None; // disable default background
                tree.default_style = style;
                continue;
            }

            let scope_names: Vec<&str> = token_color
                .scope
                .as_ref()
                .map(|scope| match *scope {
                    JsonScope::S(ref s) => s.as_str().split(',').map(|s| s.trim()).collect(),
                    JsonScope::L(ref l) => l.iter().map(|s| s.as_str()).collect(),
                })
                .unwrap();
            for name in scope_names {
                tree.insert(name, Style::from(token_color.style.clone()));
            }
        }
        Ok(tree)
    }

    fn insert(&mut self, key: &str, value: Style) {
        let keys: Vec<_> = key.split('.').collect();
        self.root.insert(&keys, value);
    }

    pub fn get(&self, key: &str) -> Style {
        let mut style = Style::empty();
        for scope_name in key.split(' ').filter(|s| !s.is_empty()) {
            let keys: Vec<_> = scope_name.split('.').collect();
            if let Some(ref s) = self.root.get(&keys) {
                style = style.overlap(s);
            }
        }
        style
    }

    pub fn style<T: AsRef<str>>(&self, keys: &[T]) -> Style {
        let mut style = Style::empty();
        for key in keys {
            style = style.overlap(&self.get(key.as_ref()));
        }
        self.default_style.overlap(&style)
    }
}

struct Node {
    value: Style,
    children: HashMap<String, Node>,
}

impl Node {
    fn new(value: Style) -> Node {
        Node {
            value,
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
            let node = self.children.entry(keys[0].to_string()).or_insert_with(
                || {
                    Node::new(Style::empty())
                },
            );
            (*node).insert(&keys[1..], value);
        }
    }

    fn get(&self, keys: &[&str]) -> Option<&Style> {
        if !keys.is_empty() {
            if let Some(node) = self.children.get(keys[0]) {
                let v = node.get(&keys[1..]);
                if v.is_some() && !v.as_ref().unwrap().is_empty() {
                    return v;
                }
            }
        }
        Some(&self.value)
    }
}

#[derive(Clone)]
pub struct Style {
    fg: Option<usize>,
    bg: Option<usize>,
    fs: Option<usize>,
}

impl From<RawStyle> for Style {
    fn from(raw_style: RawStyle) -> Self {
        let fs = raw_style.font_style.map(|s| {
            let mut fs = 0usize;
            for fs_str in s.split_whitespace() {
                match fs_str {
                    "bold" => fs |= FONTSTYLE_BOLD,
                    "italic" => fs |= FONTSTYLE_ITALIC,
                    "underline" => fs |= FONTSTYLE_UNDERLINE,
                    _ => continue,
                }
            }
            fs
        });

        Style {
            fg: raw_style.foreground,
            bg: raw_style.background,
            fs,
        }
    }
}

impl Style {
    pub fn empty() -> Style {
        Style {
            fg: None,
            bg: None,
            fs: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.fg.is_none() && self.bg.is_none() && self.fs.is_none()
    }

    pub fn overlap(&self, style: &Style) -> Style {
        let mut new = self.clone();
        if style.fg.is_some() {
            new.fg = style.fg.clone();
        }
        if style.bg.is_some() {
            new.bg = style.bg.clone();
        }
        if style.fs.is_some() {
            new.fs = style.fs.clone();
        }
        new
    }

    pub fn color(&self) -> String {
        if self.is_empty() {
            return Style::reset();
        }

        let mut props = Vec::new();
        if let Some(fs) = self.fs {
            if fs & FONTSTYLE_BOLD > 0 {
                props.push("1".to_owned());
            }
            if fs & FONTSTYLE_ITALIC > 0 {
                props.push("3".to_owned());
            }
            if fs & FONTSTYLE_UNDERLINE > 0 {
                props.push("4".to_owned());
            }
        }
        if let Some(fg) = self.fg {
            props.push(format!("38;5;{}", fg));
        }
        if let Some(bg) = self.bg {
            props.push(format!("48;5;{}", bg));
        }
        format!("\x1B[{}m", props.join(";"))
    }

    pub fn reset() -> String {
        "\x1B[0m".to_owned()
    }
}
