use onig::Regex;
use std::process::Command;

pub struct MatchResult {
    pub captures: Vec<Option<(usize, usize)>>,
}

impl MatchResult {
    pub fn start(&self) -> usize {
        assert!(!self.captures.is_empty());
        self.captures[0].unwrap().0
    }

    pub fn end(&self) -> usize {
        assert!(!self.captures.is_empty());
        self.captures[0].unwrap().1
    }
}

pub fn simple_match(pattern: &str, text: &str) -> Option<MatchResult> {
    let regex = Regex::new(pattern).unwrap();
    regex.captures(text).map(|cap| {
        let mut captures = Vec::new();
        for pos in cap.iter_pos() {
            captures.push(pos);
        }
        MatchResult {
            captures,
        }
    })
}

