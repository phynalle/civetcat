use pcre::Pcre;
use onig::Regex;

/*
struct Regex {
    src: pcre::Pcre,
}

impl Regex {
    fn new(pattern: &str) -> Regex {
        Regex { pcre::Pcre::compile(pattern) }
    }
} */

#[derive(Debug)]
pub struct RegexSet {
    srcs: Vec<Regex>,
}

impl RegexSet {
    pub fn new() -> RegexSet {
        RegexSet {
            srcs: Vec::new(),
        }
    }

    pub fn with_patterns(patterns: &[&str]) -> RegexSet {
        RegexSet {
            srcs: patterns.into_iter().map(|s| RegexSet::_compile_source(s)).collect(),
        }
    }

    fn _compile_source(pattern: &str) -> Regex {
        let emsg = format!("regex_set: compiled failed: {}", pattern);
        Regex::new(pattern).expect(&emsg)
    }

    pub fn add(&mut self, pattern: &str) {
        self.srcs.push(RegexSet::_compile_source(pattern));                
    }

    pub fn find(&self, text: &str) -> Vec<MatchResult> {
        self.srcs
            .iter()
            .enumerate()
            .filter_map(|(i, src)| {
                src.captures(text).map(|m| (i, m))
            })
            .map(|(i, m)| {
                let groups = (1..m.len())
                    .filter_map(|i| m.pos(i))
                    .collect();
                let (start, end) = m.pos(0).unwrap();
                MatchResult {
                    index: i,
                    start,
                    end,
                    groups: groups,
                }
            })
            .collect()
    }
}

pub struct MatchResult {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub groups: Vec<(usize, usize)>,
}