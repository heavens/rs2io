use crate::packet::error::PacketError;
use std::cmp::min;
use std::io;

/// A specialized buffer that allows for reading and writing data at bit-level granularity.
pub struct PacketBit {
    /// Current bit write position within the current byte (0-7)
    writer_bit_pos: usize,
    /// Current bit read position within the current byte (0-7)
    reader_bit_pos: usize,
    /// Underlying buffer for data storage
    buffer: Box<[u8]>,
}

impl PacketBit {
    /// Precalculated bit masks for efficient bit operations (2^n - 1)
    const BIT_MASKS: [u32; 32] = [
        0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff,
        0x3fff, 0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff,
        0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff
    ];

    const BITS_PER_BYTE: usize = 8;

    pub fn new() -> Self {
        Self::with_capacity(32)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            writer_bit_pos: 0,
            reader_bit_pos: 0,
            buffer: vec![0u8; capacity].into_boxed_slice(),
        }
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let mut buffer = vec![0u8; data.len()].into_boxed_slice();
        buffer.copy_from_slice(data);

        Self {
            writer_bit_pos: 0,
            reader_bit_pos: 0,
            buffer,
        }
    }

    pub fn pbit(&mut self, value: u32) -> Result<(), PacketError> {
        self.pbits(1, value)
    }

    pub fn pbits(&mut self, len: usize, value: u32) -> Result<(), PacketError> {
        self.write_bits(self.writer_bit_pos, len, value)?;
        self.writer_bit_pos += len;
        Ok(())
    }

    pub fn write_bits(&mut self, index: usize, len: usize, value: u32) -> Result<(), PacketError> {
        const BITS_PER_BYTE: usize = 8;
        const BITS_PER_INT: usize = 32;
        const MASK_BITS_PER_BYTE: usize = BITS_PER_BYTE - 1;

        let mut remaining = len;
        let mut byte_index = index >> 3;
        let mut bit_index = index & MASK_BITS_PER_BYTE;

        while remaining > 0 {
            let n = min(BITS_PER_BYTE - bit_index, remaining);
            let shift = (BITS_PER_BYTE - (bit_index + n)) & MASK_BITS_PER_BYTE;
            let mask = (1 << n) - 1u32;

            let mut v = self.buffer[byte_index];
            v = v & !(mask << shift) as u8;
            v |= (((value >> (remaining - n)) & mask) as u8) << shift;
            self.buffer[byte_index] = v;

            remaining -= n;
            byte_index += 1;
            bit_index = 0;
        }

        Ok(())
    }

    pub fn gbits(&mut self, bits: usize) -> Result<u32, PacketError> {
        let value = self.read_bits(self.reader_bit_pos as u32, bits as u32);
        self.reader_bit_pos += bits;
        value
    }

    pub fn read_bits(&mut self, index: u32, len: u32) -> Result<u32, PacketError> {
        const BITS_PER_BYTE: usize = 8;
        const BITS_PER_INT: usize = 32;
        const MASK_BITS_PER_BYTE: usize = BITS_PER_BYTE - 1;

        let mut value = 0u32;
        let mut remaining = len as usize;
        let mut byte_index = index >> 3;
        let mut bit_index = index as usize & MASK_BITS_PER_BYTE;

        while remaining > 0 {
            let n = min(BITS_PER_BYTE - bit_index, remaining);

            let shift = (BITS_PER_BYTE - (bit_index + n)) & MASK_BITS_PER_BYTE;
            let mask = (1 << n) - 1;

            let v = self.buffer[byte_index as usize];

            value <<= n;
            value |= ((v >> shift) & mask as u8) as u32;

            remaining -= n;
            byte_index += 1;
            bit_index = 0;
        }

        Ok(value)
    }

    /// Reset the reader position to the beginning of the buffer.
    pub fn reset_reader(&mut self) {
        self.reader_bit_pos = 0;
    }

    /// Ensure the buffer has enough capacity.
    fn ensure_capacity(&mut self, required_capacity: usize) -> io::Result<()> {
        if required_capacity <= self.buffer.len() {
            return Ok(());
        }

        let mut new_capacity = self.buffer.len();
        while new_capacity < required_capacity {
            new_capacity *= 2;
        }

        let mut new_buffer = vec![0u8; new_capacity].into_boxed_slice();
        new_buffer[..self.buffer.len()].copy_from_slice(&self.buffer);
        self.buffer = new_buffer;

        Ok(())
    }

    pub fn as_slice(&self) -> &[u8] {
        let end = self.writer_byte_position();
        &self.buffer[..end]
    }

    pub fn write_to_packet(&self, packet: &mut dyn PacketWriter) -> Result<(), PacketError> {
        packet.put_slice(self.as_slice())?;
        Ok(())
    }

    pub fn writer_byte_position(&self) -> usize {
        self.writer_bit_pos >> 3
    }

    pub fn writer_bit_position(&self) -> usize {
        self.writer_bit_pos
    }

    pub fn reader_byte_position(&self) -> usize {
        self.reader_bit_pos >> 3
    }

    pub fn reader_bit_position(&self) -> usize {
        self.reader_bit_pos
    }
}

pub trait PacketWriter {
    fn put_slice(&mut self, data: &[u8]) -> Result<(), PacketError>;
}
