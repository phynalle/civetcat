use clap::{App, Arg};

const ABOUT: &str = "civet colorizes your inputs";

pub fn initialize() -> App<'static, 'static> {
    App::new("civet")
        .version(crate_version!())
        .author(crate_authors!())
        .about(ABOUT)
        .arg(Arg::with_name("number-nonblank").short("b").help(
            "number non-blank output lines",
        ))
        .arg(Arg::with_name("number").short("n").help(
            "number all output lines",
        ))
        .arg(Arg::with_name("squeeze-blank").short("s").help(
            "squeeze multiple blank line into one",
        ))
        .arg(Arg::with_name("raw-control-chars").short("r").help(
            "output raw control characters",
        ))
        .arg(
            Arg::with_name("theme")
                .value_name("theme")
                .long("theme")
                .short("t")
                .help("change color styles"),
        )
        .arg(Arg::with_name("file").multiple(true))
}
