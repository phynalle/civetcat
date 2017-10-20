#[derive(Clone)]
pub struct StrPiece<'a> {
    s: &'a str,
    pos: usize,
    len: usize,
}

impl<'a> StrPiece<'a> {
    pub fn new(s: &'a str) -> StrPiece {
        StrPiece::with_bounds(s, 0, s.len())
    }

    pub fn with_bounds(s: &'a str, pos: usize, len: usize) -> StrPiece {
        assert!(pos <= s.len() && pos + len <= s.len());
        StrPiece {
            s,
            pos,
            len,
        }
    }
    
    pub fn start(&self) -> usize { self.pos }
    pub fn end(&self) -> usize { self.pos + self.len }
    pub fn len(&self) -> usize { self.len }

    pub fn substr(&self, pos: usize, len: usize) -> StrPiece<'a> {
        assert!(pos + len <= self.len);
        StrPiece::with_bounds(self.s, self.pos + pos, len)
    }

    pub fn prefix(&self, len: usize) -> StrPiece<'a> {
        assert!(len <= self.len);
        self.substr(0, len)
    }

    pub fn suffix(&self, pos: usize) -> StrPiece<'a> {
        assert!(pos <= self.len);
        self.substr(pos, self.len-pos)
    }
    
    pub fn get(&self) -> &'a str {
        &self.s[self.pos..self.pos+self.len]
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn test_creation() {
        let s = StrPiece::new("hello, world");
        let s2 = s.substr(0, 5);
        let s3 = s.prefix(5);
        let s4 = s.suffix(7);
        let s5 = s.substr(4, 4);

        assert_eq!(s2.get(), "hello");
        assert_eq!(s2.get(), s3.get());
        assert_eq!(s4.get(), "world");
        assert_eq!(s5.get(), "o, w");
    }
}
