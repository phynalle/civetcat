use syntax::raw_rule::RawRule;

use _generated;

pub trait Loader {
    fn load(&self, &str) -> Option<RawRule>;
}

pub struct GlobalSourceLoader;

impl Loader for GlobalSourceLoader {
    fn load(&self, source_name: &str) -> Option<RawRule> {
        RawRule::from_str(_generated::retrieve_syntax(source_name)).ok()
    }
}
