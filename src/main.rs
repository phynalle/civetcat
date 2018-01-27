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

mod lazy;
mod lang;
mod theme;
mod app;
mod syntax;
mod style;
mod colorizer;
mod civet;
mod _generated;

use civet::Civet;

fn main() {
    Civet::new().run();
}
