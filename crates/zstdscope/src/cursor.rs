use crate::ZstdError;

pub(crate) struct Cursor<'a> {
    input: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self { input, position: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn remaining(&self) -> usize {
        self.input.len().saturating_sub(self.position)
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, ZstdError> {
        Ok(self.read_array::<1>()?[0])
    }

    pub(crate) fn read_u16_le(&mut self) -> Result<u16, ZstdError> {
        Ok(u16::from_le_bytes(self.read_array::<2>()?))
    }

    pub(crate) fn read_u24_le(&mut self) -> Result<u32, ZstdError> {
        let bytes = self.read_array::<3>()?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]))
    }

    pub(crate) fn read_u32_le(&mut self) -> Result<u32, ZstdError> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    pub(crate) fn read_u64_le(&mut self) -> Result<u64, ZstdError> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    pub(crate) fn skip(&mut self, len: usize) -> Result<(), ZstdError> {
        self.take(len).map(|_| ())
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ZstdError> {
        let bytes = self.take(N)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);
        Ok(array)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ZstdError> {
        let start = self.position;
        let end = start
            .checked_add(len)
            .ok_or_else(|| ZstdError::ArithmeticOverflow {
                offset: self.offset_for_error(),
            })?;
        let remaining = self.remaining();
        let input = self.input;
        let bytes = input
            .get(start..end)
            .ok_or_else(|| ZstdError::UnexpectedEof {
                offset: self.offset_for_error(),
                needed: len,
                remaining,
            })?;

        self.position = end;
        Ok(bytes)
    }

    fn offset_for_error(&self) -> u64 {
        u64::try_from(self.position).unwrap_or(u64::MAX)
    }
}
