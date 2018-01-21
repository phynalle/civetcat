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
    use super::*;

    fn helloworld<'a>() -> StrPiece<'a> {
        StrPiece::new("hello, world") // 12 bytes
    }

    #[test]
    fn intialize() {
        assert_eq!(helloworld().get(), "hello, world");
        assert_eq!(StrPiece::with("hello, world", 3, 8).get(), "lo, worl");
    }

    #[test]
    fn substr() {
        let s = helloworld();
        assert_eq!(s.substr(0, 5).get(), "hello");
        assert_eq!(s.substr(7, 5).get(), "world");
        assert_eq!(s.substr(4, 4).get(), "o, w");
        assert_eq!(s.substr(2, 8).substr(2, 5).get(), "o, wo");
    }

    #[test]
    fn remove_prefix() {
        let s = helloworld();
        assert_eq!(
            {
                let mut s = s;
                s.remove_prefix(5);
                s
            }.get(),
            s.substr(5, 7).get()
        );
    }
}
