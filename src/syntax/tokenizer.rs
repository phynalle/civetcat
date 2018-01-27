use std::mem::replace;
use std::rc::Rc;

use syntax::rule::{self, CaptureGroup, Grammar, Rule, Type};
use syntax::regex::{self, Regex};
use syntax::str_piece::StrPiece;

enum BestMatchResult {
    Pattern(rule::MatchResult),
    End(regex::MatchResult),
    None,
}

pub struct Tokenizer {
    state: State,
    //TODO: Rc<Grammar> should be reduced to &Grammar
    grammar: Rc<Grammar>,
    tokengen: TokenGenerator,
}

impl Tokenizer {
    pub fn new(grammar: &Rc<Grammar>) -> Tokenizer {
        let mut tokenizer = Tokenizer {
            state: State::new(),
            grammar: Rc::clone(grammar),
            tokengen: TokenGenerator::new(),
        };

        tokenizer.state.push(&grammar.rule(grammar.root_id()), None);
        tokenizer
    }

    pub fn tokenize_line(&mut self, line: &str) -> Vec<Token> {
        let line_str = StrPiece::new(line);
        let while_not_matched = {
            let top = self.state.top();
            match top.rule.display() {
                Type::BeginWhile => top.match_expr(line_str).is_none(),
                _ => false,
            }
        };
        if while_not_matched {
            self.state.pop();
        }

        self.tokenize_string(line_str);
        self.tokengen.take()
    }

    fn tokenize_string<'b>(&mut self, mut text: StrPiece<'b>) {
        while let Some(pos) = self.tokenize_next(text) {
            let offset = text.start();
            text.remove_prefix(pos - offset);
        }
    }

    fn tokenize_next<'b>(&mut self, text: StrPiece<'b>) -> Option<usize> {
        match self.best_match(text) {
            BestMatchResult::Pattern(m) => {
                let pos = (m.caps.start(), m.caps.end());
                self.generate_token(pos.0);

                // TOOD: remove repetitive codes below
                let rule = self.grammar.rule(m.rule);
                rule.do_match(|r| {
                    self.state.push(&rule, None);
                    self.process_capture(text, &m.caps.captures, &r.captures);
                    self.generate_token(pos.1);
                    self.state.pop();
                });
                rule.do_beginend(|r| {
                    let s = replace_backref(r.end_expr.clone(), text, &m.caps);
                    self.state.push(&rule, Some(s));
                    self.process_capture(text, &m.caps.captures, &r.begin_captures);
                    self.generate_token(pos.1);

                    self.state.push_scope(r.content_name.clone());
                });
                rule.do_beginwhile(|r| {
                    let s = replace_backref(r.while_expr.clone(), text, &m.caps);
                    self.state.push(&rule, Some(s));
                    self.process_capture(text, &m.caps.captures, &r.begin_captures);
                    self.generate_token(pos.1);
                });
                Some(pos.1)
            }
            BestMatchResult::End(m) => {
                let pos = (m.start(), m.end());
                self.generate_token(pos.0);
                {
                    let rule = self.state.top().rule.clone();
                    rule.do_beginend(|r| {
                        self.state.pop_addition_scope();
                        self.process_capture(text, &m.captures, &r.end_captures);
                    });
                }
                self.generate_token(pos.1);
                self.state.pop();
                Some(pos.1)
            }
            BestMatchResult::None => {
                self.generate_token(text.end());
                None
            }
        }
    }


    fn best_match<'b>(&mut self, text: StrPiece<'b>) -> BestMatchResult {
        let state = self.state.top();
        let pattern_match = state
            .rule
            .collect_matches(text)
            .into_iter()
            .filter(|x| x.caps.start() != x.caps.end())
            .min_by_key(|x| x.caps.start());
        let end_match = match state.rule.display() {
            Type::BeginEnd => state.match_expr(text),
            _ => None,
        };

        match (pattern_match, end_match) {
            (None, None) => BestMatchResult::None,
            (None, Some(e)) => BestMatchResult::End(e),
            (Some(p), None) => BestMatchResult::Pattern(p),
            (Some(p), Some(e)) => {
                if e.start() <= p.caps.start() {
                    BestMatchResult::End(e)
                } else {
                    BestMatchResult::Pattern(p)
                }
            }
        }
    }

    fn process_capture<'b>(
        &mut self,
        text: StrPiece<'b>,
        captured: &[Option<(usize, usize)>],
        capture_group: &CaptureGroup,
    ) {
        if captured.len() == 0 {
            return;
        }

        let mut st: Vec<(usize, usize)> = Vec::new();
        for (i, cap) in captured.into_iter().enumerate() {
            if let Some(pos) = *cap {
                if let Some(weak_rule) = capture_group.0.get(&i) {
                    let (capture_start, capture_end) = pos;
                    let capture_len = capture_end - capture_start;
                    let capture_text = text.substr(capture_start - text.start(), capture_len);

                    loop {
                        let prev_end = match st.last() {
                            Some(e) if e.1 <= capture_start => e.1,
                            _ => break,
                        };

                        self.generate_token(prev_end);
                        st.pop();
                        self.state.pop();
                    }

                    self.generate_token(capture_text.start());

                    let rule = weak_rule.upgrade().unwrap();
                    if rule.has_match() {
                        self.state.push(&rule, None);
                        self.tokenize_string(capture_text);
                        self.state.pop();
                        continue;
                    }

                    st.push((capture_start, capture_end));
                    self.state.push(&rule, None);
                }
            }
        }

        while !st.is_empty() {
            self.generate_token(st[st.len() - 1].1);
            st.pop();
            self.state.pop();
        }
    }

    fn generate_token(&mut self, pos: usize) {
        self.tokengen.generate(pos, &self.state)
    }
}

struct RuleState {
    rule: Rule,
    expr: Option<Regex>,
}

impl RuleState {
    fn match_expr<'a>(&self, text: StrPiece<'a>) -> Option<regex::MatchResult> {
        self.expr.as_ref().and_then(|expr| expr.find(text))
    }
}

struct State {
    st: Vec<RuleState>,
    scopes: Vec<Vec<Option<String>>>,
}

impl State {
    fn new() -> State {
        State {
            st: Vec::new(),
            scopes: Vec::new(),
        }
    }

    fn top(&self) -> &RuleState {
        assert!(!self.st.is_empty());
        self.st.iter().rev().nth(0).unwrap()
    }

    fn push(&mut self, rule: &Rule, expr: Option<String>) {
        self.st.push(RuleState {
            rule: rule.clone(),
            expr: expr.map(|s| Regex::new(&s)),
        });
        self.scopes.push(vec![rule.name()]);
    }

    fn push_scope(&mut self, scope: Option<String>) {
        self.scopes.iter_mut().rev().nth(0).unwrap().push(scope)
    }

    fn pop_addition_scope(&mut self) {
        self.scopes.iter_mut().rev().nth(0).unwrap().drain(1..);
    }

    fn pop(&mut self) {
        assert!(!self.st.is_empty());
        self.st.pop();
        self.scopes.pop();
    }

    fn scopes(&self) -> Vec<String> {
        self.scopes
            .iter()
            .flat_map(|v| v.iter())
            .filter_map(|s| s.clone())
            .collect()
    }

    #[allow(dead_code)]
    fn depth(&self) -> usize {
        self.st.len()
    }
}

struct TokenGenerator {
    pos: usize,
    tokens: Vec<Token>,
}

impl TokenGenerator {
    fn new() -> TokenGenerator {
        TokenGenerator {
            pos: 0,
            tokens: Vec::new(),
        }
    }

    fn generate(&mut self, end: usize, state: &State) {
        if self.pos < end {
            let token = self.generate_token(end, state);
            self.tokens.push(token);
        }
    }

    fn generate_token(&mut self, end: usize, state: &State) -> Token {
        assert!(self.pos < end);
        let start = self.pos;
        self.pos = end;
        Token {
            start,
            end,
            scopes: state.scopes(),
        }
    }

    fn take(&mut self) -> Vec<Token> {
        self.pos = 0;
        replace(&mut self.tokens, Vec::new())
    }
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub scopes: Vec<String>,
}

fn replace_backref<'a>(mut s: String, text: StrPiece<'a>, m: &regex::MatchResult) -> String {
    for (i, cap) in m.captures.iter().enumerate().skip(1) {
        if let Some(ref cap) = *cap {
            let old = format!("\\{}", i);
            let new = {
                let sub_offset = cap.0 - text.start();
                let sub_len = cap.1 - cap.0;
                text.substr(sub_offset, sub_len)
            };

            s = s.replace(&old, &new);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    use syntax::raw_rule::RawRule;
    use syntax::loader::Loader;
    use syntax::rule::GrammarBuilder;

    macro_rules! tokens {
        ( $( $start:expr, $end:expr, $( $scope:expr ),* );* ) => (
            {
                let tokens = vec![
                $( Token {
                    start: $start,
                    end: $end,
                    scopes: vec! [ $( $scope.to_owned() ),* ],
                }), *];
                tokens
            }
       );
    }

    macro_rules! tokenize_eq {
        ( $rule:expr, $text:expr, $tokens:expr )
            => ( assert_eq!(tokenize($rule, $text), $tokens) );
    }

    struct EmptySourceLoader;

    impl Loader for EmptySourceLoader {
        fn load(&self, _: &str) -> Option<RawRule> {
            None
        }
    }

    fn tokenizer(rule_text: &str) -> Tokenizer {
        let rule_text = format!(r#"{{ "patterns": [{}] }}"#, rule_text);
        let rawrule = RawRule::from_str(&rule_text);
        assert!(rawrule.is_ok());

        let rawrule = rawrule.unwrap();
        let grammar = GrammarBuilder::new(rawrule, Box::new(EmptySourceLoader)).build();
        let grammar = Rc::new(grammar);
        Tokenizer::new(&grammar)
    }

    fn tokenize(rule_text: &str, text: &str) -> Vec<Token> {
        let mut tok = tokenizer(rule_text);
        tok.tokenize_line(text)
    }

    #[test]
    fn tokenize_match() {
        tokenize_eq!(
            r#"{ "match": "(hello|world)", "name": "greet.test" }"#,
            "hello, world",
            tokens!(0, 5, "greet.test"; 5, 7, ; 7, 12, "greet.test")
        );
    }

    #[test]
    fn tokenize_beginend() {
        tokenize_eq!(
            r#"{ "begin": "\\(", "end": "\\)" }"#,
            "  (coco is fun! XD) ",
            tokens!(0, 2, ; 2, 3, ; 3, 18, ; 18, 19, ; 19, 20, )
        );

        tokenize_eq!(
            r#"{ "begin": "\\(", "end": "\\)", "name": "parens" }"#,
            "  (coco is fun! XD) ",
            tokens!(0, 2, ; 2, 3, "parens"; 3, 18, "parens"; 18, 19, "parens"; 19, 20, )
        );

        tokenize_eq!(
            r#"{ "begin": "\\(", "end": "\\)", "contentName": "parens.content" }"#,
            "  (coco is fun! XD) ",
            tokens!(0, 2, ; 2, 3, ; 3, 18, "parens.content"; 18, 19, ; 19, 20, )
        );

        tokenize_eq!(
            r#"{ "begin": "\\(", "end": "\\)", "name": "parens",
                 "contentName": "parens.content" }"#,
            "  (coco is fun! XD) ",
            tokens!(0, 2, ;
                    2, 3, "parens";
                    3, 18, "parens", "parens.content";
                    18, 19, "parens";
                    19, 20, )
        );

        // end containing backref
        tokenize_eq!(
            r#"{ "begin": "hello, (\\w+)", "end": "bye, \\1", "name": "greet" }"#,
            "Oh, hello, civet! nice to meet you. bye, civet.",
            tokens!(0, 4, ; 4, 16, "greet"; 16, 36, "greet"; 36, 46, "greet"; 46, 47, )
        );

        tokenize_eq!(
            r#"{ "begin": "123", "end": "$", "name": "123",
                "patterns": [
                    {"match": "1", "name": "1"},
                    {"match": "2", "name": "2"},
                    {"match": "3", "name": "3"}
                ]}"#,
            "123 0983614725",
            tokens!(
                0, 3, "123";
                3, 7, "123";
                7, 8, "123", "3";
                8, 9, "123";
                9, 10, "123", "1";
                10, 12, "123";
                12, 13, "123", "2";
                13, 14, "123")
        );

        tokenize_eq!(
            r#"{ "begin": "hello, (\\w+)", "end": "bye, \\1", "name": "greet" }"#,
            "Oh, hello, civet! nice to meet you. bye, civet.",
            tokens!(0, 4, ; 4, 16, "greet"; 16, 36, "greet"; 36, 46, "greet"; 46, 47, )
        );
    }

    #[test]
    fn tokenize_beginwhile() {}

    #[test]
    fn tokenize_capture_without_pattern() {
        let rule_def = r#"{
                "match": "(\\()\\s*(\\w*)\\s*(,)\\s*(\\w*)\\s*(\\))",
                "captures": {
                    "0": { "name": "pair" },
                    "1": { "name": "open" },
                    "2": { "name": "word.first" },
                    "3": { "name": "delim" },
                    "4": { "name": "word.second" },
                    "5": { "name": "close" }
                }
            }"#;
        let texts = vec!["(,)", "( x , y )"];
        let expects =
            vec![
                tokens!(0, 1, "pair", "open"; 1, 2, "pair", "delim"; 2, 3, "pair", "close"),
                tokens!(0, 1, "pair", "open"; 1, 2, "pair"; 2, 3, "pair", "word.first";
                    3, 4, "pair"; 4, 5, "pair", "delim"; 5, 6, "pair"; 6, 7, "pair", "word.second";
                    7, 8, "pair"; 8, 9, "pair", "close"),
            ];

        for (text, expect) in texts.into_iter().zip(expects) {
            tokenize_eq!(rule_def, text, expect)
        }
    }

    #[test]
    fn tokenize_capture_with_pattern() {
        tokenize_eq!(
            // This rule is taken from toml
            r#"{
                "match":"^\\s*(\\[)([^\\[\\]]*)(\\])",
                "name":"table",
                "captures":{
                    "1":{ "name":"punctuation" },
                    "2":{ "patterns":[ {
                        "match":"[^\\s.]+", "name":"name"
                    } ] },
                    "3":{ "name":"punctuation" }
                }
            }"#,
            "[  table  ]",
            tokens!(0, 1, "table", "punctuation"; 1, 3, "table"; 3, 8, "table", "name";
                    8, 10, "table"; 10, 11, "table", "punctuation")
        );
    }

    #[test]
    fn backref() {
        let re = Regex::new("#IF_(\\w+)");
        let s = StrPiece::new("   #IF_BLOCK   ");
        assert_eq!(
            &replace_backref("#END_\\1".into(), s, &re.find(s).unwrap()),
            "#END_BLOCK"
        );

        let re = Regex::new(
            "LOCATION:\\s*([\\w\\d]+)\\s*,\\s*([\\w\\d]+)\\s*,\\s*([\\w\\d]+)\\s*;",
        );
        let s = StrPiece::new("LOCATION: Jeju, Ulsan, Seoul;");
        assert_eq!(
            &replace_backref(
                "I lived in \\1, \\2, and \\3!".into(),
                s,
                &re.find(s).unwrap(),
            ),
            "I lived in Jeju, Ulsan, and Seoul!"
        );
    }
}
