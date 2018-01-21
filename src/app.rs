use clap::{App, Arg};

const ABOUT: &str = "civet colorizes your inputs";

pub fn initialize() -> App<'static, 'static> {
    App::new("civet")
        .version(crate_version!())
        .author(crate_authors!())
        .about(ABOUT)
        .arg(Arg::with_name("number").short("n").help("show line number"))
        .arg(Arg::with_name("lang").value_name("lang").long("lang").short("l").help("set default languages"))
        .arg(Arg::with_name("theme").value_name("theme").long("theme").short("t").help("set theme"))
        .arg(Arg::with_name("file").multiple(true))
}
