use std::{self, result};
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use atty;

use app;
use colorizer::LineColorizer;
use lang;
use theme;
use error::Error;
use _generated;

type Result<T> = result::Result<T, Error>;

static EXECUTABLE_NAME: &'static str = "cv";

pub struct Civet {
    ll: lang::LangLoader,
    args: Arguments,
    supported: Supported,
}

impl Civet {
    pub fn new() -> Civet {
        let supported = Supported::new();
        let args = parse_arguments(&supported);

        Civet {
            ll: lang::LangLoader::new(),
            args,
            supported,
        }
    }

    pub fn run(self) {
        let stdout = std::io::stdout();
        if self.args.options.print_supported {
            self.supported.print();
            std::process::exit(0);
        }

        for file_name in &self.args.file_names {
            let mut w = Writer::new(stdout.lock(), &self.args.options);
            if file_name == "-" {
                w.copy(std::io::stdin()).unwrap();
            } else if let Err(e) = self.write_file(&file_name, &mut w) {
                print_error(&format!("{}: {}", file_name, e))
            }
        }
    }

    fn write_file<'a, W: Write, T: AsRef<str>>(
        &self,
        file_name: T,
        writer: &mut Writer<'a, W>,
    ) -> Result<()> {
        let options = &self.args.options;
        let file = File::open(file_name.as_ref())?;
        let path = Path::new(file_name.as_ref());
        let grammar = if options.raw_control_chars {
            path.extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext| lang::identify(ext))
                .map(|ln| self.ll.load_grammar(ln))
        } else {
            None
        };

        match grammar {
            Some(g) => {
                let mut lc = LineColorizer::new(theme::load(options.theme), &g);
                writer.write(file, |s| Cow::Owned(lc.process_line(s)))
            }
            None => writer.copy(file),
        }
    }
}

struct Writer<'a, W: Write> {
    inner: W,
    options: &'a Options,
}

impl<'a, W: Write> Writer<'a, W> {
    fn new(inner: W, options: &'a Options) -> Writer<'a, W> {
        Writer { inner, options }
    }

    fn copy<R: Read>(&mut self, r: R) -> Result<()> {
        self.write(r, |s| Cow::Borrowed(s)).map_err(|e| e.into())
    }

    fn write<R, F>(&mut self, r: R, mut f: F) -> Result<()>
    where
        R: Read,
        F: for<'b> FnMut(&'b str) -> Cow<'b, str>,
    {
        let mut line_num = 1;
        let mut prev_blank = false;
        let mut reader = BufReader::new(r);

        loop {
            let mut line = String::new();
            let blank_line = match reader.read_line(&mut line)? {
                0 => break,
                _ => line == "\n" || line == "\r\n",
            };

            if self.options.squeeze_blank && prev_blank && blank_line {
                continue;
            }
            prev_blank = blank_line;

            if self.options.display_number {
                if self.options.number_nonblack && blank_line {
                    self.inner.write_fmt(format_args!("      \t")).unwrap();
                } else {
                    self.inner
                        .write_fmt(format_args!("{:6}\t", line_num))
                        .unwrap();
                    line_num += 1;
                };
            }
            self.inner.write_fmt(format_args!("{}", f(&line))).unwrap();
        }
        self.inner.flush().map_err(|e| e.into())
    }
}

struct Arguments {
    options: Options,
    file_names: Vec<String>,
}

fn parse_arguments(supported: &Supported) -> Arguments {
    let matches = app::initialize().get_matches();

    let mut options = Options {
        display_number: false,
        number_nonblack: matches.occurrences_of("number-nonblank") > 0,
        squeeze_blank: matches.occurrences_of("squeeze-blank") > 0,
        raw_control_chars: matches.occurrences_of("raw-control-chars") > 0,
        print_supported: matches.occurrences_of("supported") > 0,

        theme: theme::default(),
    };

    options.display_number |= matches.occurrences_of("number") > 0;
    options.display_number |= matches.occurrences_of("number-nonblank") > 0;
    options.raw_control_chars |= atty::is(atty::Stream::Stdout);

    if !options.print_supported {
        if let Some(theme_name) = matches.value_of("theme") {
            match supported.find_theme(&theme_name) {
                Ok(th) => options.theme = th,
                Err(e) => {
                    print_error(&format!("{}: {}", e, theme_name));
                    supported.print();
                    std::process::exit(1);
                }
            }
        }
    }

    let file_names = matches
        .values_of("file")
        .map(|values| values.map(|v| v.to_owned()).collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["-".to_owned()]);
    Arguments {
        options,
        file_names,
    }
}

#[derive(Clone)]
struct Options {
    display_number: bool,
    number_nonblack: bool,
    squeeze_blank: bool,
    raw_control_chars: bool,
    print_supported: bool,
    theme: _generated::Theme,
}

fn print_error(err: &str) {
    let exe = get_exe_name();
    let mut stderr = std::io::stderr();
    writeln!(&mut stderr, "{}: {}", exe, err).unwrap();
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

struct Supported {
    themes: Vec<(String, _generated::Theme)>,
}

impl Supported {
    pub fn new() -> Supported {
        Supported {
            themes: _generated::themes().to_vec(),
        }
    }

    pub fn find_theme(&self, name: &str) -> Result<_generated::Theme> {
        let name = name.to_lowercase();
        self.themes
            .iter()
            .find(|&&(ref n, _)| name == n.to_lowercase())
            .map(|&(_, th)| th)
            .ok_or(Error::UnsupportedTheme)
    }

    pub fn print(&self) {
        println!("Supported Themes");
        for &(ref name, _) in &self.themes {
            println!(" * {}", name);
        }
    }
}
