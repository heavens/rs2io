use std::fmt::Debug;
use std::io::{ErrorKind, Read};
use std::ops::{Deref, DerefMut, Index, Range, RangeFrom, RangeInclusive, RangeTo};
use std::{cmp, io};
use crate::error::{error, PacketError};

macro_rules! g {
    ($this:ident, $value_size:literal, $value_expr:expr) => {{

        let cap = $this.len();
        let pos = $this.pos;
        if pos + $value_size > cap {
            return error(
                format!(
                    "expected pos + size_in_bytes < limit. (pos: {}, cap: {})",
                    pos, cap
                )
            );
        }

        let slice = unsafe { *($this.bytes[pos..pos + $value_size].as_ptr() as *const [_; $value_size]) };
        $this.pos += $value_size;
        Ok($value_expr(slice))
    }};
}

macro_rules! p {
    ($this:tt,  $value:tt) => {{
        let pos = $this.pos;
        let slice_len = $value.len();
        let buf_len = $this.bytes.len();
        if pos + slice_len >= buf_len {
            $this.bytes.resize((slice_len + buf_len) * 2, 0u8);
        }

        $this.bytes.deref_mut()[pos..pos + slice_len].copy_from_slice($value);
        $this.pos += slice_len;
    }};
}

#[derive(Clone)]
pub struct Packet {
    bytes: Vec<u8>,
    pos: usize,
}

impl Packet {
    /// Creates a new byte buffer whose contents are initialized with 0.
    pub fn new(capacity: usize) -> Self {
        let buf = vec![0u8; capacity];
        Self {
            bytes: buf,
            pos: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            bytes: vec![0u8; 0],
            pos: 0,
        }
    }

    /// Resizes the buffer to satisfy the `new_len`, filling the allocated memory with the
    /// provided value.
    pub(crate) fn resize(&mut self, new_len: usize) {
        self.bytes.resize(new_len, 0u8);
    }

    pub(crate) fn check_for_space(&mut self, space: usize) {
        if self.readable() < space {
            self.resize(space * 2);
        }
    }

    /// Clears the buffer by setting both the read and write position to 0.
    pub fn clear(&mut self) {
        self.pos = 0;
    }
}

impl From<Vec<u8>> for Packet {
    fn from(value: Vec<u8>) -> Self {
        Packet {
            pos: 0,
            bytes: value,
        }
    }
}

impl From<&[u8]> for Packet {
    fn from(value: &[u8]) -> Self {
        Packet {
            pos: 0,
            bytes: value.to_vec(),
        }
    }
}

impl<const N: usize> From<&[u8; N]> for Packet {
    fn from(value: &[u8; N]) -> Self {
        value.to_vec().into()
    }
}

impl Index<RangeInclusive<usize>> for Packet {
    type Output = [u8];

    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        let start = *index.start();
        let end = *index.end();
        if end <= start {
            return &[];
        }
        &self.deref()[start..=end]
    }
}

impl Index<RangeTo<usize>> for Packet {
    type Output = [u8];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        let end = index.end;
        if end == 0 {
            return &[];
        }
        &self.deref()[..end]
    }
}

impl Index<RangeFrom<usize>> for Packet {
    type Output = [u8];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        let start = index.start;
        if start >= self.len() {
            return &[];
        }
        &self.deref()[start..]
    }
}

impl Deref for Packet {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl DerefMut for Packet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

impl AsRef<[u8]> for Packet {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl Debug for Packet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ByteBuffer")
            .field("buf", &self.bytes)
            .finish()
    }
}

impl Packet {
    /// Attempts to return an unsigned byte from the reader, incrementing the position by `1` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g1(&mut self) -> Result<u8, PacketError> {
        g!(self, 1, u8::from_be_bytes)
    }

    /// Attempts to return a signed byte from the reader, incrementing the position by `1` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g1s(&mut self) -> Result<i8, PacketError> {
        g!(self, 1, i8::from_be_bytes)
    }

    /// Attempts to return a signed short from the reader, incrementing the position by `2` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g2s(&mut self) -> Result<i16, PacketError> {
        g!(self, 2, i16::from_be_bytes)
    }

    /// Attempts to return an unsigned short from the reader, incrementing the position by `2` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g2(&mut self) -> Result<u16, PacketError> {
        g!(self, 2, u16::from_be_bytes)
    }

    /// Attempts to return a 24-bit unsigned integer from the reader, incrementing the position by `3` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn get_u24(&mut self) -> Result<usize, PacketError> {
        if self.available(3) {
            self.pos += 3;
            Ok((self.bytes[self.pos - 3] as usize) << 16
                | (self.bytes[self.pos - 2] as usize) << 8
                | self.bytes[self.pos - 1] as usize)
        } else {
            error(format!("expected at least 3 bytes, but only {} were available", self.readable()))
        }
    }

    /// Attempts to return a signed integer from the reader, incrementing the position by
    /// `4` if successful otherwise returning if not enough bytes remain.
    pub fn g4s(&mut self) -> Result<i32, PacketError> {
        g!(self, 4, i32::from_be_bytes)
    }

    /// Attempts to return an unsigned integer from the reader, incrementing the position by `4` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g4(&mut self) -> Result<u32, PacketError> {
        g!(self, 4, u32::from_be_bytes)
    }

    /// Attempts to return a signed long from the reader, incrementing the position by `8` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g8s(&mut self) -> Result<i64, PacketError> {
        g!(self, 8, i64::from_be_bytes)
    }

    /// Attempts to return an unsigned long from the reader, incrementing the position by `8` if successful. Otherwise
    /// an error is returned if not enough bytes remain.
    pub fn g8(&mut self) -> Result<u64, PacketError> {
        g!(self, 8, u64::from_be_bytes)
    }

    /// Performs a conditional "smart" read, returning a signed short or unsigned byte depending on the value of the
    /// next readable byte and increasing the position based on the integer type read. Otherwise an error is returned
    /// if not enough bytes remain.
    pub fn gsmart(&mut self) -> Result<usize, PacketError> {
        if let Some(next) = self.peek() {
            if next > 127 {
                return self.g2s().map(|value| value as usize);
            }
            return self.g1().map(|value| value as usize);
        }
        error("expected at least one byte for get_smart but none were available.".to_string())
    }

    /// Tries to read a null-terminated string (c-string) from the reader, returning an error if the operation could not complete. The reader
    /// position is incremented based on the width of the string read.
    pub fn gjstr(&mut self) -> Result<String, PacketError> {
        let mut contents = Vec::new();
        while let Some(next) = self.peek() {
            if next == 0 {
                break;
            }

            contents.push(self.g1()?);
        }

        let str_len = contents.len();
        if let Ok(string) = String::from_utf8(contents) {
            self.pos += str_len + 1;
            Ok(string)
        } else {
            error("attempted to read bytes that are not of valid utf-8 encoding.".to_string())
        }
    }

    /// Sets the position at the specified index within the internal buffer.
    pub fn set_pos(&mut self, index: usize) -> Result<(), PacketError> {
        if index >= self.pos {
            return error(format!("attempted to set cursor at invalid position. expected index < len (index: {}, len: {})", index, self.bytes.len()))
        }
        self.pos = index;
        Ok(())
    }

    /// Returns an optional value for the next byte available without incrementing the buffer's position, otherwise returning `None`.
    pub fn peek(&self) -> Option<u8> {
        if self.readable() == 0 {
            return None;
        }

        self.bytes.get(self.pos).copied()
    }

    /// Increments the reader position by `count` bytes. If the specified count causes the position to overflow then it's resized to
    /// `remaining`.
    pub fn skip(&mut self, bytes: usize) {
        self.pos += cmp::min(bytes, self.readable());
    }

    /// Returns the reader's current position within the buffer.
    pub fn read_pos(&self) -> usize {
        self.pos
    }

    /// Returns the amount of readable bytes remaining expressed as `capacity` minus `read_pos`.
    ///
    /// ## Safety
    ///
    /// This operation is safe from potential would-be overflows and returns a value of
    /// `0` in the event of one occurring.
    pub fn readable(&self) -> usize {
        let (value, overflowed) = self.pos.overflowing_sub(self.pos);
        if overflowed {
            return 0;
        }

        value
    }

    /// Returns the amount of writable bytes remaining expressed as `capacity` minus `write_pos`.
    ///
    /// ## Safety
    ///
    /// This operation is safe from potential would-be overflows and returns a value of
    /// `0` in the event of one occurring.
    pub fn writable(&self) -> usize {
        let (value, overflowed) = self.bytes.capacity().overflowing_sub(self.pos);
        if overflowed {
            return 0;
        }

        value
    }

    /// Returns `true` if no readable bytes remain. Shorthand for `self.remaining() == 0`.
    pub fn is_empty(&self) -> bool {
        self.readable() == 0
    }

    /// Returns `true` if at least `count` bytes are remaining in the reader.
    pub fn available(&self, count: usize) -> bool {
        self.readable() >= count
    }
}

impl Index<Range<usize>> for Packet {
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.bytes[index.start..index.end]
    }
}

impl Read for Packet {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = self.pos;
        let mut values = buf.len();
        let len = values;
        while values > 0 {
            buf[len - values] = self.g1().map_err(|reason| io::Error::new(ErrorKind::Other, format!("unable to read bytes at current pos: {:?}", reason)))?;
            values -= 1;
        }
        Ok(self.pos - pos)
    }
}

impl Iterator for Packet {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(byte) = self.g1() {
            Some(byte)
        } else {
            None
        }
    }
}

impl Packet {
    /// Writes an unsigned byte value into the buffer, incrementing the position by `1`.
    pub fn p1(&mut self, value: u8) {
        let slice = &u8::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes a signed byte value into the buffer, incrementing the position by `1`.
    pub fn p1s(&mut self, value: i8) {
        let slice = &i8::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes a signed short value into the buffer, incrementing the position by `2`.
    pub fn p2s(&mut self, value: i16) {
        let slice = &i16::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes an unsigned short value into the buffer, incrementing the position by `2`.
    pub fn p2(&mut self, value: u16) {
        let slice = &u16::to_be_bytes(value);
        p!(self, slice)
    }

    pub fn p3(&mut self, value: u32) {
        // Expand the underlying buffer to make room for at least 3 more bytes.
        if self.pos + 3 >= self.len() {
            self.resize(3);
        }

        self.pos += 3;
        self.bytes[self.pos - 3] = (value >> 16) as u8;
        self.bytes[self.pos - 2] = (value >> 8) as u8;
        self.bytes[self.pos - 1] = value as u8;
    }

    /// Writes a signed int value into the buffer, incrementing the position by `4`.
    pub fn p4s(&mut self, value: i32) {
        let slice = &i32::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes an unsigned int value into the buffer, incrementing the position by `4`.
    pub fn p4(&mut self, value: u32) {
        let slice = &u32::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes an unsigned int value into the buffer, incrementing the position by `8`.
    pub fn p8(&mut self, value: u64) {
        let slice = &u64::to_be_bytes(value);
        p!(self, slice)
    }

    /// Writes a null-terminated string value into the buffer, incrementing the position by
    /// `value.len() + 1`.
    pub fn pjstr(&mut self, value: impl AsRef<str>) {
        let bytes: &[u8] = value.as_ref().as_bytes();
        p!(self, bytes);
        self.p1(0);
    }

    pub fn gjstr2(&mut self, value: impl AsRef<str>) {
        self.p1(0);
        self.pjstr(value);
    }

    /// Conditionally writes a signed byte if `n <= 127` otherwise writes an unsigned short. The
    /// position is incremented relative to the type written.
    pub fn psmart(&mut self, value: usize) {
        if value <= 127 {
            self.p1s(value as i8);
        } else {
            self.p2(value as u16);
        }
    }

    /// Increases the capacity of the underlying buffer to be capable of storing at least `new_cap`
    /// amount of items.
    ///
    /// ### Note
    /// Attempting to set the capacity of the writer to anything smaller than the current capacity
    /// will result in a no-op.
    pub fn grow(&mut self, new_cap: usize) {
        let old_cap = self.bytes.capacity();
        if new_cap > old_cap {
            self.bytes.resize(new_cap - old_cap, 0);
        }
    }

    /// Returns the capacity of the underlying buffer denoting the amount of items the buffer is
    /// capable of holding.
    pub fn len(&self) -> usize {
        self.bytes.capacity()
    }

    /// Writes a raw byte slice to the buffer. The position is incremented based on the len of the slice written.
    pub fn put_slice(&mut self, slice: &[u8]) {
        p!(self, slice);
    }

    /// Allocates an array capable of holding the copied contents of this writer.
    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use crate::bytes::Packet;

    #[test]
    fn test_read() {

        let mut packet = Packet::new(10);
        packet.p4(100);
        packet.p4(5);
        packet.clear();
        let val = packet.g4().expect("should read bytes");
        let val2 = packet.g4().expect("should read next byte.");

        let readable = packet.readable();
        println!("{val}, {val2}, {readable}");
    }
}
