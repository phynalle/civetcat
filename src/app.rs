use clap::{App, Arg};

const ABOUT: &'static str = "civet colorizes your inputs";

pub fn initialize() -> App<'static, 'static> {
    App::new("civet")
        .version(crate_version!())
        .author(crate_authors!())
        .about(ABOUT)
        .arg(Arg::with_name("number")
             .short("n")
             .help("show line number"))
        .arg(Arg::with_name("file")
             .multiple(true))
}
