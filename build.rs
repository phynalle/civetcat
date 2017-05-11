#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::io::{Read, Write};

use std::io::Result;
use std::fs::File;

macro_rules! skeleton {
    () => (
"use std::collections::HashMap;
use std::io::Result;
use syntax::Syntax;
use tokenizer::Grammar;

{}

lazy_static! {{
    pub static ref EXT_LANG_MAP: HashMap<&'static str, &'static str> = {{
        let mut m = HashMap::new();
{}
        m
    }};
}}

pub fn _load_grammar(lang: &str) -> Result<Grammar> {{
    match lang {{
{}
        _ => panic!(\"undefined language: {{}}\", lang),
    }}
}}

{}
");
}

fn main() {
    let config = load_config().unwrap();
    let mut raw = String::new();
    let mut ext = String::new();
    let mut lg = String::new();
    let mut func = String::new();

    for lang in config.languages {
        let _raw = raw_name(&lang.name);
        let _fn = func_name(&lang.name);

        raw.push_str(&format!("const {}: &'static str = \"{}\";\n", _raw, read_file(&lang.path)));
        lg.push_str(&format!("        \"{}\" => {}(),\n", lang.name, _fn));
        func.push_str(&format_func(&_fn, &_raw));
        for e in lang.extensions {
            ext.push_str(&format!("        m.insert(\"{}\", \"{}\");\n", e, lang.name));
        }
    }

    let mut f = File::create("src/_generated.rs").unwrap();
    let _ = f.write_fmt(format_args!(skeleton!(), raw, ext, lg, func));
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    languages: Vec<Language>,
}

#[derive(Deserialize, Debug, Clone)]
struct Language {
    name: String,
    path: String,
    extensions: Vec<String>,
}

fn load_config() -> Result<Config> {
    let file = File::open("config.json")?;
    Ok(serde_json::from_reader(file).unwrap())
}

fn read_file(path: &str) -> String {
    let mut s = String::new();
    let _ = File::open(path).unwrap().read_to_string(&mut s);
    s.replace("\\", "\\\\").replace("\"", "\\\"")
}

fn raw_name(lang: &str) -> String {
    format!("RAW_{}_SYNTAX", lang.to_uppercase())
}

fn func_name(lang: &str) -> String {
    format!("_load_{}_grammar", lang.to_lowercase())
}

fn format_func(_fn: &str, _func: &str) -> String {
    format!(
"fn {}() -> Result<Grammar> {{
    let syntax = Syntax::from_str({})?;
    Ok(syntax.compact())
}}
", _fn, _func)
}