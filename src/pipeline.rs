use std::fs::File;
use std::io::Read;

use colorizer::ScopeTree;
use colorizer::TextColorizer;
use colorizer::Settings;
use syntax::Tokenizer;

struct Pipeline {
    scopes: ScopeTree,
}

impl Pipeline {
    fn new(scopes: ScopeTree) -> Pipeline {
        Pipeline { scopes: scopes }
    }
}

pub fn do_pipeline(filename: &str) {
    let scope = ScopeTree::create("themes/Kimbie_dark.json").unwrap();
    let tok = Tokenizer::create("syntaxes/rust.tmLanguage.json").unwrap();
    let mut toker = tok.instance();
    let text = load_text(filename);

    for line in text.lines() {
        let tokens = toker.tokenize(&line);
        let mut v: Vec<_> = tokens.into_iter()
            .map(|t| {
                t.captures.iter()
                    .map(|&(begin, end, ref name)| (begin, end, scope.get(&name)) )
                    .filter(|&(_, _, ref s)| s.is_some() && !s.as_ref().unwrap().is_empty() )
                    .map(|(begin, end, s)| (begin, end, s.unwrap()))
                    .collect::<Vec<_>>()
            })
            .filter(|ref v| !v.is_empty())
            .flat_map(|v| v.into_iter())
            .collect();
        v.sort_by(|&(ax, ay, _), &(bx, by, _)| (ax, ay).cmp(&(bx, by)) );

        let mut s: String = line.to_owned();
        for p in TextColorizer::process(&v) {
            s.insert_str(p.0, &p.1);
        }
        println!("{}", s);
    }
}

fn load_text(filename: &str) -> String {
    let mut buf = String::new();
    let _ = File::open(filename).unwrap().read_to_string(&mut buf);
    buf
}