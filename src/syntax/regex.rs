use onig::{self, RegexOptions, Syntax, Region, SearchOptions};
use syntax::str_piece::StrPiece;

pub struct Regex {
    re: onig::Regex,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        let option = RegexOptions::REGEX_OPTION_NONE;
        let re = onig::Regex::with_options_and_encoding(pattern, option, Syntax::default())
            .expect(&format!("cannot compile pattern: {}", pattern));
        Regex { re }
    }

    pub fn find<'a>(&self, text: StrPiece<'a>) -> Option<MatchResult> {
        let offset = text.start();

        let mut options = SearchOptions::SEARCH_OPTION_NONE;
        if offset > 0 {
            options |= SearchOptions::SEARCH_OPTION_NOTBOL;
        }
        if text.full_text().len() > text.end() {
            options |= SearchOptions::SEARCH_OPTION_NOTEOL;
        }

        let mut region = Region::new();
        self.re
            .search_with_options(text.get(), 0, text.len(), options, Some(&mut region))
            .map(|_| {
                let captures = (0..region.len())
                    .map(|pos| {
                        region.pos(pos).map(
                            |(start, end)| (start + offset, end + offset),
                        )
                    })
                    .collect();
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
