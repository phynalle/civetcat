use std::ops::Deref;

#[derive(Clone, Copy)]
pub struct StrPiece<'a> {
    s: &'a str,
    pos: usize,
    len: usize,
}

impl<'a> StrPiece<'a> {
    pub fn new(s: &'a str) -> StrPiece {
        StrPiece::with(s, 0, s.len())
    }

    pub fn with(s: &'a str, pos: usize, len: usize) -> StrPiece {
        assert!(pos + len <= s.len());
        StrPiece { s, pos, len }
    }

    pub fn start(&self) -> usize {
        self.pos
    }

    pub fn end(&self) -> usize {
        self.pos + self.len
    }

    pub fn substr(&self, pos: usize, len: usize) -> StrPiece<'a> {
        assert!(self.len >= pos);
        StrPiece::with(self.s, self.pos + pos, len)
    }

    pub fn remove_prefix(&mut self, n: usize) {
        assert!(self.len >= n);
        self.pos += n;
        self.len -= n;
    }

    pub fn get(&self) -> &'a str {
        &self.s[self.pos..self.pos + self.len]
    }

    pub fn full_text(&self) -> &'a str {
        self.s
    }
}

impl<'a> Deref for StrPiece<'a> {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_creation() {
        let s = StrPiece::new("hello, world");
        let s2 = s.substr(0, 5);
        let s5 = s.substr(4, 4);

        assert_eq!(s2.get(), "hello");
        assert_eq!(s2.get(), s3.get());
        assert_eq!(s4.get(), "world");
        assert_eq!(s5.get(), "o, w");
    }
}
