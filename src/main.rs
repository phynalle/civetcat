#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate atty;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate onig;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod lazy;
mod lang;
mod theme;
mod app;
mod syntax;
mod style;
mod colorizer;
mod civet;
mod error;
mod _generated;

use civet::Civet;

fn main() {
    Civet::new().run();
}
