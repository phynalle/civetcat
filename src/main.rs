#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate pcre;
extern crate onig;

#[macro_use]
extern crate lazy_static;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::iter::Iterator;
use std::path::Path;

mod lang;
mod parser;
mod syntax;
mod colorizer;
mod pipeline;
mod tokenizer;
mod _generated;

mod syntax2;

use pipeline::Pipeline;
static EXECUTABLE_NAME: &'static str = "cv";

#[derive(Copy, Clone)]
struct Options {
    display_number: bool,
}

struct Parsed {
    options: Options,
    file_names: Vec<String>,
}

fn main() {
    let rule = syntax2::grammar::load_grammars("./syntaxes/rust.json");
    let c = syntax2::rule::Compiler::new();
    let grammar = c.compile(&mut rule.unwrap());

    let mut text = String::new();
    match std::io::stdin().read_to_string(&mut text) {
        Ok(0) | Err(_) => (),
        Ok(_) => grammar.tokenize_test(&text),
    }
    /*
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = parse_options(args);




    match result {
        Ok(parsed) => run(parsed),
        Err(e) => {
            print_error(&e);
            let _ = writeln!(&mut std::io::stderr(),
                             "usage: {} [-n] [file ...]",
                             get_exe_name());
        }
    }*/
}

fn run(mut parsed: Parsed) {
    let ll = lang::LangLoader::new();

    if parsed.file_names.is_empty() {
        parsed.file_names.push("-".to_owned());
    }

    for file_name in &parsed.file_names {
        let mut printer = ColorPrinter::new(parsed.options);
        if file_name == "-" {
            printer.print(std::io::stdin(), |s| s.to_owned());
        } else {
            match File::open(file_name.clone()) {
                Ok(file) => {
                    let path = Path::new(file_name);
                    let grammar = path.extension()
                        .and_then(|ext| ext.to_str())
                        .and_then(|ext| lang::identify(ext))
                        .map(|ln| ll.load_grammar(ln));

                    match grammar {
                        Some(g) => {
                            printer.print(file, |s| {
                                let mut pl = Pipeline::new(g.clone());
                                pl.process_line(s)
                            })
                        }
                        None => printer.print(file, |s| s.to_owned()),
                    }
                }
                Err(e) => {
                    print_error(&format!("{}: {}", file_name, e));
                }
            }
        }
    }
}

fn print_error(err: &str) {
    let exe = get_exe_name();
    let mut stderr = std::io::stderr();
    let _ = writeln!(&mut stderr, "{}: {}", exe, err);
}

fn get_exe_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| {
                      p.file_name()
                          .and_then(|s| s.to_str())
                          .map(|s| s.to_string())
                  })
        .unwrap_or_else(|| EXECUTABLE_NAME.to_owned())
}

fn parse_options(mut flags: Vec<String>) -> Result<Parsed, String> {
    let mut options = Options { display_number: false };
    while !flags.is_empty() {
        {
            let first = &flags[0];
            if !first.starts_with('-') || first.len() == 1 {
                break;
            }
        }

        let s = flags.remove(0);
        for c in s[1..].chars() {
            if c == 'n' {
                options.display_number = true;
            } else {
                return Err(format!("illegal option -- {}", c));
            }
        }
    }

    let parsed = Parsed {
        options: options,
        file_names: flags,
    };
    Ok(parsed)
}

struct ColorPrinter {
    options: Options,
}

impl ColorPrinter {
    fn new(options: Options) -> ColorPrinter {
        ColorPrinter { options }
    }

    fn print<R: Read, F: Fn(&str) -> String>(&mut self, r: R, f: F) {
        let stdout = std::io::stdout();
        let mut o = stdout.lock();
        let mut line_num = 1;
        let mut reader = BufReader::new(r);
        loop {
            let mut line = String::new();
            let (text, r) = match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(mut n) => {
                    if line.ends_with('\n') {
                        n -= 1;
                        if line.ends_with("\r\n") {
                            n -= 1;
                        }
                    }
                    line.split_at(n)
                }
                Err(e) => panic!("{}", e),
            };

            let text = f(text);
            if self.options.display_number {
                let _ = o.write_fmt(format_args!("{:6}\t", line_num));
            }
            let _ = o.write_fmt(format_args!("{}{}", text, r));
            line_num += 1;
        }
        let _ = o.flush();
    }
}
