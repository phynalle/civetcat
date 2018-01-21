use _generated::{Theme, _load_theme};
use style::StyleTree;

pub fn default() -> Theme {
    Theme::Monokai
}

pub fn load(theme: Theme) -> StyleTree {
    _load_theme(theme).unwrap()
}
