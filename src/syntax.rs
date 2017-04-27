use std::io::{Result, Read};
use std::fs::File;

use std::rc::Rc;
use std::cell::{Cell, RefCell};
use pcre::Pcre;

use regex::{self, Regex, RegexSet};
use serde_json;

use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum Pattern {
    Root(Syntax),
    Include(Include),
    Match(Match),
    Block(Block),
}

impl Pattern {
    // fn expression(&self, repos: &HashMap<String, Pattern>) -> String {
    //     match *self {
    //         Pattern::Include(ref p) => p.expression(repos),
    //         Pattern::Block(ref p) => p.expression(),
    //         Pattern::Match(ref p) => p.expression(),
    //         _ => "".to_owned(),
    //     }
    // }

    fn refer<'a, 'b: 'a>(&'b self, repos: &'a HashMap<String, Pattern>) -> &'a Pattern {
        match *self {
            Pattern::Include(ref p) => p.refer(&repos),
            _ => &self,
        }        
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Include {
    include: String,
}
impl Include {
    fn name(&self) -> String {
        if self.include.starts_with("#") {
            self.include.chars().skip(1).collect()
        } else {
            self.include.clone()
        }
    }


    // fn expression(&self, repos: &HashMap<String, Pattern>) -> String {
    //     let root = self.name();
    //     let mut current = root.clone();
    //     loop {
    //         let pattern = repos.get(&current).unwrap();
    //         match *pattern {
    //             Pattern::Include(ref p) => {
    //                 let target = p.name();
    //                 if target == root {
    //                     panic!("Cycle Error");
    //                 }
    //                 current = target;             
    //             },
    //             Pattern::Match(ref p) => {
    //                 return p.expression();
    //             },
    //             Pattern::Block(ref p) => {
    //                 return p.expression();
    //             }
    //             _ => return String::new()
    //         }
    //     }
    // }

    fn refer<'a>(&self, repos: &'a HashMap<String, Pattern>) -> &'a Pattern {
        let root = self.name();
        let mut current = root.clone();
        loop {
            let pattern = repos.get(&current).unwrap();
            match *pattern {
                Pattern::Include(ref p) => {
                    let target = p.name();
                    if target == root {
                        panic!("Cycle Error");
                    }
                    current = target;             
                }
                Pattern::Match(_) | Pattern::Block(_) => {
                    return &pattern;
                }
                _ => panic!("Unreachable")
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Block {
    #[serde(rename = "name")]
    scope: Option<String>,
    begin: String,
    end: String,
    begin_captures: Captures,
    end_captures: Captures,
    patterns: Option<Vec<Pattern>>,
}

impl Block {
    // fn expression(&self) -> String {
    //     self.begin.clone()
    // }

    fn match_begin<'a>(&self, mut line: &'a str) -> Option<(Vec<Token>, &'a str)> {
        let mut tokens = Vec::new();

        let mut pcre = Pcre::compile(&self.begin).unwrap();
        if let Some(m) = pcre.exec(&line) {
            let mut captures = Vec::new();
            if let Some(ref scope) = self.scope {
                captures.push((m.group_start(0), m.group_end(0), scope.clone()));
            }
            if let Some(ref caps) = self.begin_captures {
                for i in 1..m.string_count() {
                    if let Some(ref cap) = caps.get(&i.to_string()) {
                        captures.push((m.group_start(i), m.group_end(i), cap.name.clone()));
                    }
                }
            }
            tokens.push(Token {
                text: m.group(0).to_string(),
                captures: captures
            });
            line = &line[m.group_end(0)..];
            Some((tokens, line))
        } else {
            None
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct Match {
    #[serde(rename = "name")]
    scope: Option<String>,
    #[serde(rename = "match")]
    pattern: String,
    captures: Captures,
}

impl Match {
    // fn expression(&self) -> String {
    //     self.pattern.clone()
    // }

    fn tokenize<'a>(&self, mut line: &'a str) -> Option<(Vec<Token>, &'a str)> {
        let mut tokens = Vec::new();

        let mut pcre = Pcre::compile(&self.pattern).unwrap();
        if let Some(m) = pcre.exec(&line) {
            let mut captures = Vec::new();
            if let Some(ref scope) = self.scope {
                captures.push((m.group_start(0), m.group_end(0), scope.clone()));
            }
            if let Some(ref caps) = self.captures {
                for i in 1..m.string_count() {
                    if let Some(ref cap) = caps.get(&i.to_string()) {
                        captures.push((m.group_start(i), m.group_end(i), cap.name.clone()));
                    }
                }
            }
            tokens.push(Token {
                text: m.group(0).to_string(),
                captures: captures
            });
            line = &line[m.group_end(0)..];
            Some((tokens, line))
        } else {
            None
        }
    }
}


#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct Syntax {
    name: String,
    scope_name: String,
    file_types: Vec<String>,
    patterns: Vec<Pattern>,
    repository: HashMap<String, Pattern>,
    version: String,
}

#[derive(Deserialize, Debug, Clone)]
struct Capture {
    name: String
}

type Captures = Option<HashMap<String, Capture>>;

impl Syntax {
    // fn compile(&self) {
    //     // Needed information
    //     // global (name, pattern(begin in block)) mapping
    //     // local (index, name) mapping

    //     let mut repos = HashMap::new();

    //     for (name, pattern) in &self.repository{
    //         repos.insert(name.clone(), pattern.clone());
    //     }
    //     for pattern in &self.patterns {
    //         let x = match *pattern {
    //             Pattern::Include(ref r) => r.expression(&repos),
    //             Pattern::Match(ref r) => r.pattern.clone(),
    //             Pattern::Block(ref r) => r.begin.clone(),
    //             _ => "C".to_owned(),
    //         };
    //     }
    // }

}

struct Tokenizer<'a> {
    root: &'a Pattern,
    stack: Stack<'a>,
}

impl<'a> Tokenizer<'a> {
    fn tokenize(&mut self, text: &str) {
        let mut tokens: Vec<Token> = Vec::new();
        self.stack.push(&self.root);

        // let mut stack = Stack::new(&root);
        for line in text.lines() {
            let mut line = line;
            while !line.is_empty() {
                let result = self.tokenize_line(&line);
                if result.is_none() {
                    break;
                }
                let (x, remaining) = result.unwrap();
                tokens.extend_from_slice(&x);
                line = remaining;
            }
        }

        for token in &tokens {
            println!("{}", token.text);
            println!("{:?}", token.captures);
        }
    }

    fn repository(&self) -> &'a HashMap<String, Pattern> {
        if let Pattern::Root(ref r) = *self.root {
            &r.repository
        } else {
            panic!("Unreachable!!!");
        }
    }

/*
Line 1. 
One of pattern in root is selected.

the most matched pattern is Match
    make token
    repeat next remaining str

the most matched pattern is Block
    Push into stack
        repeat
    if End pattern is matched
    Pop stack

the most Include
    Go to pattern
    repeat

Root 
    push into stack
        iterator lines 
            match line
    pop

*/
    fn tokenize2<'b>(&mut self, patterns: &'a Vec<Pattern>, line: &'b str) -> Option<(Vec<Token>, &'b str)> {
        for pat in patterns {
            let pat = pat.refer(self.repository());
            if let &Pattern::Match(ref p) = pat {
                let result = p.tokenize(line); 
                if result.is_none() {
                    continue;
                }
                return result;
            } else if let &Pattern::Block(ref p) = pat {
                let result = p.match_begin(line);
                if result.is_none() {
                    continue;
                }
                println!("Push: {}", p.begin);
                self.stack.push(&pat);
                return result;
            }
        }
        None
    }

    fn tokenize_line<'b>(&mut self, mut line: &'b str) -> Option<(Vec<Token>, &'b str)> {
        match *self.stack.top() {
            Pattern::Block(ref r) => {
                let mut tokens = Vec::new();

                if let Some(ref pats) = r.patterns {
                   let result = self.tokenize2(&pats, line); 
                   if let Some((ref v, ref text)) = result {
                       tokens.extend_from_slice(&v);
                       line = text;
                   }
                }

                if let Some(m) = Pcre::compile(&r.end).unwrap().exec(&line) {
                    println!("End: {}~{}", r.begin, r.end);
                    if let Some(scope) = self.stack.top_scope() {
                        let token = Token {
                            text: m.group(0).to_string(),
                            captures: vec![(m.group_start(0), m.group_end(0), scope)]
                        };
                        tokens.push(token);
                        line = &line[m.group_end(0)..];
                    }
                    self.stack.pop();
                }
                if !tokens.is_empty() {
                    return Some((tokens, line));
                }
            }
            Pattern::Root(ref r) => {
                return self.tokenize2(&r.patterns, line);
            }
            _ => panic!("Unreachable!")
        };
        None
    }
}

#[derive(Clone, Debug)]
struct Token {
   text: String,
   captures: Vec<(usize, usize, String)>
}

// struct Repository {
//     inner: HashMap<String, Rc<CompiledPattern>>
// }

// impl Repository {
//     fn new(base: &HashMap<String, Pattern>) {
//         for (name, pattern) in base {
            
//         }

//     }
// }

// struct Ruler {
//     repos: HashMap<String, CompiledPattern>, // name, pattern 
// }

// #[derive(Debug)]
// struct PatternMap {
//     names: Vec<Pattern>,
//     re: RegexSet,
// }

fn load_text() -> String {
    let mut buf = String::new();
    let _ = File::open("src/main.rs").unwrap().read_to_string(&mut buf);
    buf
}

pub fn parse_syntax() -> Result<()> {
    let file = File::open("syntaxes/rust.tmLanguage.json")?;
    let syntax: Syntax = serde_json::from_reader(file)?;

    let text = load_text();
    let root = Pattern::Root(syntax.clone());
    let mut tokenizer = Tokenizer { root: &root, stack: Stack::new() };
    tokenizer.tokenize(&text);
    // root.tokenize(&text);
    /*
    for pattern in &de.patterns {
        println!("pattern: {:?}", pattern);
    }
    for (k, v) in &de.repository {
        println!("{}: {:?}", k, v);
    }*/
    /*
    let mut builder = regex::RegexSetBuilder::new(&[
        r"\w+",
        r"\d+",
        r"\pL+",
        r"foo",
        r"bar",
        r"barfoo",
        r"foobar",
    ]);
    builder.multi_line(true);
    // builder.ignore_whitespace(true);
    builder.unicode(true);
    let set = builder.build().unwrap();
    let matches: Vec<_> = set.matches("   foobar23").into_iter().collect();
    println!("{:?}", matches);
*/

    Ok(())
}

struct Stack<'a> {
    scopes: Vec<&'a Pattern>,
}

impl<'a> Stack<'a> {
    fn new() -> Stack<'a> {
        Stack {
            scopes: vec![],
        }
    }

    fn push(&mut self, pat: &'a Pattern) {
        self.scopes.push(pat);
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn top(&self) -> &'a Pattern {
        self.scopes[self.scopes.len() - 1]
    }

    fn top_scope(&self) -> Option<String> {
        if let Pattern::Block(ref r) = *self.top() {
            r.scope.clone()
        } else {
            None
        }
    }
}


