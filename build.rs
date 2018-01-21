#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;


#[path = "src/syntax/raw_rule.rs"]
mod raw_rule;

use std::io::{Read, Write, Result};
use std::fs::File;

use raw_rule::test;

macro_rules! skeleton {
    () => (
"use std::collections::HashMap;
use std::io::Result;
use style::{{StyleTree, load_theme}};

#[derive(Clone, Copy)]
pub enum Theme {{
{}
}}

{}

lazy_static! {{
    pub static ref EXT_LANG_MAP: HashMap<&'static str, &'static str> = {{
        let mut m = HashMap::new();
{}
        m
    }};
}}

pub fn retrieve_syntax(lang: &str) -> &'static str {{
    match lang {{
{}
        _ => panic!(\"undefined language: {{}}\", lang),
    }}
}}

/*
pub fn _load_grammar(lang: &str) -> Result<Grammar> {{
    match lang {{
{}
        _ => panic!(\"undefined language: {{}}\", lang),
    }}
}}
*/

pub fn _load_theme(theme: Theme) -> Result<StyleTree> {{
    match theme {{
{}
    }}
}}

{}


lazy_static! {{
    pub static ref LANGS: Vec<String> = {{
        let mut v = Vec::new();
{}

        v
    }};

    pub static ref THEMES: Vec<(String, Theme)> = {{
        let mut v = Vec::new();
{}

        v
    }};
}}

pub fn langs() -> &'static LANGS {{
    &LANGS
}}

pub fn themes() -> &'static THEMES {{
    &THEMES
}}

");
}

fn main() {
    let config = load_config().unwrap();
    let mut raw = String::new();
    let mut ext = String::new();
    let mut lg = String::new();
    let mut func = String::new();
    let mut lt = String::new();
    let mut syn_mat = String::new();
    let mut theme_def = String::new();
    let mut langs = String::new();
    let mut themes = String::new();

    for path in config.languages {
        let lang = load_language(&path).expect(&format!("{} not found", path));
        let _raw = raw_syntax_name(&lang.name);
        let _fn = syntax_func_name(&lang.name);

        raw.push_str(&format!(
            "const {}: &str = \"{}\";\n",
            _raw,
            read_file(&path)
        ));
        syn_mat.push_str(&format!("        \"{}\" => {},\n", lang.scope_name, _raw));
        lg.push_str(&format!("        \"{}\" => {}(),\n", lang.scope_name, _fn));
        // func.push_str(&format!("{}\n", gen_load_syntax_func(&lang.name)));

        for e in lang.file_types {
            ext.push_str(&format!(
                "        m.insert(\"{}\", \"{}\");\n",
                e,
                lang.scope_name
            ));
        }
        langs.push_str(&format!("       v.push(\"{}\".to_owned());\n", lang.name));
    }

    for theme in config.themes {
        let _raw = raw_theme_name(&theme.name);
        let _fn = theme_func_name(&theme.name);
        raw.push_str(&format!(
            "const {}: &str = \"{}\";\n",
            _raw,
            read_file(&theme.path)
        ));
        theme_def.push_str(&format!("    {},\n", theme.name));
        lt.push_str(&format!("        Theme::{} => {}(),\n", theme.name, _fn));
        func.push_str(&format!("{}\n", gen_load_theme_func(&theme.name)));
        themes.push_str(&format!("       v.push((\"{}\".to_owned(), Theme::{}));\n", theme.name, theme.name));
    }

    let mut f = File::create("src/_generated.rs").unwrap();
    let _ = f.write_fmt(format_args!(
        skeleton!(),
        theme_def,
        raw,
        ext,
        syn_mat,
        lg,
        lt,
        func,
        langs,
        themes
    ));
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    languages: Vec<String>,
    themes: Vec<Theme>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Language {
    name: String,
    scope_name: String,
    file_types: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct Theme {
    name: String,
    path: String,
}

fn load_config() -> Result<Config> {
    let file = File::open("config.json")?;
    Ok(serde_json::from_reader(file).unwrap())
}

fn load_language(path: &str) -> Result<Language> {
    let file = File::open(path)?;
    let lang: Language = serde_json::from_reader(file)?;
    Ok(lang)
}

fn read_file(path: &str) -> String {
    let mut s = String::new();
    let _ = File::open(path).unwrap().read_to_string(&mut s);

    test(&s);
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\t", "")
        .replace("\n", "")
}

fn raw_syntax_name(lang: &str) -> String {
    format!("RAW_{}_SYNTAX", lang.to_uppercase())
}

fn syntax_func_name(lang: &str) -> String {
    format!("_load_{}_grammar", lang.to_lowercase())
}

// fn gen_load_syntax_func(lang: &str) -> String {
//     format!(
//         "fn {}() -> Result<Grammar> {{
//     load_grammar({})
// }}
// ",
//         &syntax_func_name(lang),
//         &raw_syntax_name(lang)
//     )
// }

fn raw_theme_name(theme: &str) -> String {
    format!("RAW_{}_THEME", theme.to_uppercase())
}

fn theme_func_name(theme: &str) -> String {
    format!("_load_{}_theme", theme.to_lowercase())
}

fn gen_load_theme_func(theme: &str) -> String {
    format!(
        "fn {}() -> Result<StyleTree> {{
    load_theme({})
}}
",
        &theme_func_name(theme),
        &raw_theme_name(theme)
    )
}
