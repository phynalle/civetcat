use _generated::{_load_theme, Theme};
use style::StyleTree;

pub fn default() -> Theme {
    Theme::Monokai
}

pub fn load(theme: Theme) -> StyleTree {
    _load_theme(theme).unwrap()
}
