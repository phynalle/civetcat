use _generated::{Theme, _load_theme};
use style::StyleTree;

pub fn load() -> StyleTree {
    _load_theme(Theme::Monokai).unwrap()
}
