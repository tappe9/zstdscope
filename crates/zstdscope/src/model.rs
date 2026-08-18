#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub offset: u64,
    pub length: u64,
}

impl ByteSpan {
    pub fn end(&self) -> Option<u64> {
        self.offset.checked_add(self.length)
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}
