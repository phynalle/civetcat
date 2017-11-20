#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate onig;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
extern crate atty;

use std::fs::File;
use std::rc::Rc;
use std::io::BufReader;
use std::io::prelude::*;
use std::iter::Iterator;
use std::path::Path;
use atty::Stream;

mod lazy;
mod lang;
mod theme;
mod app;
mod parser;
mod syntax;
mod colorizer;
mod pipeline;
mod _generated;

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
    run(parse_options());
}

fn run(mut parsed: Parsed) {
    let ll = lang::LangLoader::new();

    if parsed.file_names.is_empty() {
        parsed.file_names.push("-".to_owned());
    }

    for file_name in &parsed.file_names {
        let mut printer = Printer::new(parsed.options);
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
                            printer.print(file, |s| if atty::is(Stream::Stdout) {
                                let mut pl = Pipeline::new(theme::load(), Rc::clone(&g));
                                pl.process_line(&s)
                            } else {
                                s.to_owned()
                            });
                        }
                        None => {
                            printer.print(file, |s| s.to_owned());
                        }
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
            p.file_name().and_then(|s| s.to_str()).map(
                |s| s.to_string(),
            )
        })
        .unwrap_or_else(|| EXECUTABLE_NAME.to_owned())
}

fn parse_options() -> Parsed {
    let matches = app::initialize().get_matches();

    let mut options = Options { display_number: false };
    if matches.occurrences_of("number") > 0 {
        options.display_number = true;
    }

    let file_names = matches
        .values_of("file")
        .map(|values| values.map(|v| v.to_owned()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["-".to_owned()]);

    Parsed {
        options,
        file_names,
    }
}

struct Printer {
    options: Options,
}

impl Printer {
    fn new(options: Options) -> Printer {
        Printer { options }
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
