use onig;

pub struct Regex {
    re: onig::Regex,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        Regex {
            re: onig::Regex::new(pattern).expect(&format!("cannot compile pattern: {}", pattern)),
        }
    }

    pub fn find(&self, text: &str) -> Option<MatchResult> {
        self.re.captures(text).map(|cap| {
            let mut captures = Vec::new();
            for pos in cap.iter_pos() {
                captures.push(pos);
            }
            MatchResult { captures }
        })
    }
}

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
    Regex::new(pattern).find(text)
}

