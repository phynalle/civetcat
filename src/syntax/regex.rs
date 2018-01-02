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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! p {
        ( $x:expr, $y:expr ) => (Some(($x, $y)));
    }

    macro_rules! pv {
        ( $( $x:expr ),* ) => ( Some( vec![ $($x,)* ]) );
    }

    fn find<'a>(re: &Regex, haystack: StrPiece<'a>) -> Option<Vec<Option<(usize, usize)>>> {
        re.find(haystack).map(|m| m.captures)
    }

    #[test]
    fn match_substring_test() {
        let haystack = StrPiece::new("abc 1234");

        // the begin of substring of line may not be the begin of line.
        let re = Regex::new("^");
        assert_eq!(find(&re, haystack), pv![p!(0, 0)]);
        assert_eq!(find(&re, haystack.substr(0, 5)), pv![p!(0, 0)]);
        assert_eq!(find(&re, haystack.substr(4, 3)), None);

        // the end of substring of line may not be the end of line.
        let re = Regex::new("$");
        assert_eq!(find(&re, haystack), pv![p!(8, 8)]);
        assert_eq!(find(&re, haystack.substr(5, 3)), pv![p!(8, 8)]);
        assert_eq!(find(&re, haystack.substr(4, 3)), None);

        // capture positions must be in range of substring
        let re = Regex::new("12");
        assert_eq!(find(&re, haystack.substr(3, 2)), None);
        assert_eq!(find(&re, haystack.substr(3, 3)), pv![p!(4, 6)]);
        assert_eq!(find(&Regex::new("(?!)\\G"), haystack.substr(3, 0)), None);
    }
}
