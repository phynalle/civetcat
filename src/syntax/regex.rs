use onig;
use syntax::str_piece::StrPiece;

pub struct Regex {
    re: onig::Regex,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        Regex {
            re: onig::Regex::new(pattern).expect(&format!("cannot compile pattern: {}", pattern)),
        }
    }

    pub fn find<'a>(&self, text: StrPiece<'a>) -> Option<MatchResult> {
        self.re.captures(&text).map(|cap| {
            let mut captures = Vec::new();
            for pos in cap.iter_pos() {
                captures.push(pos);
            }
            MatchResult { captures }
        })
    }
}

#[derive(Debug)]
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
