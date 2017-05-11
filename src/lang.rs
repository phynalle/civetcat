use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use tokenizer::Grammar;
use _generated;

pub fn identify(ext: &str) -> Option<&str> {
    _generated::EXT_LANG_MAP.get(ext).cloned()
}

pub struct LangLoader { 
    grammars: RefCell<HashMap<String, Rc<Grammar>>>
}

impl LangLoader {
    pub fn new() -> LangLoader {
        LangLoader { 
            grammars: RefCell::new(HashMap::new()) 
        }
    }

    pub fn load_grammar(&self, lang: &str) -> Rc<Grammar> {
        if let Some(g) = self.grammars.borrow().get(lang) {
            return g.clone();
        }
        match _generated::_load_grammar(lang) {
            Ok(g) => {
                let g = Rc::new(g);
                self.grammars.borrow_mut().insert(lang.to_owned(), g.clone());                
                g
            }
            Err(e) => panic!("{}", e),
        } 
    }
}
