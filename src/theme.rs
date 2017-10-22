use _generated::{Theme, _load_theme};
use colorizer::ScopeTree;

pub fn load() -> ScopeTree {
    _load_theme(&Theme::KimbieDark).unwrap()
}
