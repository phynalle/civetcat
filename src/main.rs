use std::cell::Cell;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::iter::Iterator;

mod lang;
mod parser;

use lang::Highlighter;

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
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = parse_options(args);

    match result {
        Ok(parsed) => run(parsed),
        Err(e) => {
            print_error(e);
            let _ = writeln!(&mut std::io::stderr(),
                             "usage: {} [-n] [file ...]",
                             get_exe_name());
        }
    }
}

fn run(mut parsed: Parsed) {
    if parsed.file_names.is_empty() {
        parsed.file_names.push("-".to_owned());
    }

    for file_name in parsed.file_names.iter() {
        let printer = Printer::new(parsed.options);
        if file_name == "-" {
            printer.print(std::io::stdin());
        } else {
            match File::open(file_name.clone()) {
                Ok(file) => {
                    printer.print(file);
                }
                Err(e) => {
                    print_error(format!("{}: {}", file_name, e));
                }
            }
        }
    }
}

fn print_error(err: String) {
    let exe = get_exe_name();
    let mut stderr = std::io::stderr();
    let _ = writeln!(&mut stderr, "{}: {}", exe, err);
}

fn get_exe_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .unwrap_or(EXECUTABLE_NAME.to_owned())
}

fn parse_options(mut flags: Vec<String>) -> Result<Parsed, String> {
    let mut options = Options { display_number: false };
    while !flags.is_empty() {
        {
            let first = &flags[0];
            if !first.starts_with("-") || first.len() == 1 {
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

struct Printer {
    line_num: Cell<usize>,
    options: Options,
    hl: Highlighter,
}

impl Printer {
    fn new(options: Options) -> Printer {
        Printer {
            line_num: Cell::new(0),
            options: options,
            hl: Highlighter,
        }
    }

    fn print_line<T: AsRef<str>>(&self, s: T) {
        let line_num = self.line_num.get() + 1;
        self.line_num.set(line_num);

        if self.options.display_number {
            print!("{:6}\t", line_num);
        }
        println!("{}", s.as_ref());
    }

    fn print<R: Read>(&self, r: R) {
        let reader = BufReader::new(r);
        let reader = self.hl.apply(reader);

        for line in reader.lines() {
            self.print_line(line.unwrap());
        }
    }
}
