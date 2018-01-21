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
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::Iterator;
use std::path::Path;
use std::borrow::Cow;

use atty::Stream;

mod lazy;
mod lang;
mod theme;
mod app;
mod syntax;
mod style;
mod colorizer;
mod _generated;

use colorizer::LineColorizer;

static EXECUTABLE_NAME: &'static str = "cv";

#[derive(Copy, Clone)]
struct Options {
    display_number: bool,
    theme: _generated::Theme,
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
            printer.print(std::io::stdin(), |s| Cow::Borrowed(s));
        } else {
            print_file(&parsed.options, file_name.clone(), &mut printer, &ll);
        }
    }
}

fn print_file<T: AsRef<str>>(options: &Options, file_name: T, printer: &mut Printer, ll: &lang::LangLoader) {
    match File::open(file_name.as_ref()) {
        Ok(file) => {
            let path = Path::new(file_name.as_ref());
            let grammar = if atty::is(Stream::Stdout) {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .and_then(|ext| lang::identify(ext))
                    .map(|ln| ll.load_grammar(ln))
            } else {
                None
            };

            match grammar {
                Some(g) => {
                    let mut lc = LineColorizer::new(theme::load(options.theme), &g);
                    printer.print(file, |s| Cow::Owned(lc.process_line(s)));
                }
                None => printer.print(file, |s| Cow::Borrowed(s)),
            }
        }
        Err(e) => {
            print_error(&format!("{}: {}", file_name.as_ref(), e));
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

    let mut options = Options {
        display_number: false,
        theme: theme::default(),
    };

    if matches.occurrences_of("number") > 0 {
        options.display_number = true;
    }

    if let Some(theme_name) = matches.value_of("theme") {
        let themes = _generated::themes().to_vec();
        match theme_name {
            "list" => {
                println!("Supported Themes");
                for (name, _) in themes {
                    println!(" * {}", name);
                }
                std::process::exit(0);
            }
            _ => {
                let theme = {
                    let theme_name = theme_name.to_lowercase();
                    themes.into_iter()
                        .find(|&(ref name, _)| name.to_lowercase() == theme_name)
                        .map(|(_, th)| th)
                };

                match theme {
                    Some(th) => options.theme = th,
                    None => {
                        println!("Unsupported Theme: {}", theme_name);
                        std::process::exit(1);
                    }
                }

                if let Some(theme) = theme {
                    options.theme = theme;
                }
            }
        }
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

    fn print<R, F>(&mut self, r: R, mut f: F)
    where
        R: Read,
        F: for<'a> FnMut(&'a str) -> Cow<'a, str>,
    {
        let stdout = std::io::stdout();
        let mut o = stdout.lock();
        let mut line_num = 1;
        let mut reader = BufReader::new(r);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Err(e) => panic!("{}", e),
                _ => (),
            };

            if self.options.display_number {
                let _ = o.write_fmt(format_args!("{:6}\t", line_num));
            }
            let _ = o.write_fmt(format_args!("{}", f(&line)));
            line_num += 1;
        }
        let _ = o.flush();
    }
}
