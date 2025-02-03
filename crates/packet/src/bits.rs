#![allow(unused)]

use std::io;

pub struct PacketBit {
    byte_index: usize,
    bit_index: usize,
    reader_pos: usize,
    writer_pos: usize,
    buffer: Box<[u8]>,
}

impl PacketBit {
    /// Predetermined computations of base2 up to 31.
    const VALUES: [i32; 32] = [
        0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff,
        0x3fff, 0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff,
        0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff,
    ];

    /// Shift amount used to keep alignment when translating values bits to bytes and vice versa.
    const BYTE_SHIFT: usize = 3;

    /// Width of each byte in bits.
    const BITS_PER_BYTE: usize = 1 << 3;

    pub fn new() -> Self {
        Self {
            byte_index: 0,
            bit_index: 0,
            reader_pos: 0,
            writer_pos: 0,
            buffer: vec![0u8; 32].into_boxed_slice(),
        }
    }

    /// Writes a boolean-type value to the bit buffer.
    pub fn pbool(&mut self, value: bool) -> io::Result<()> {
        const REQUIRED_SPACE: usize = 1;
        let byte_value = if value { 1 } else { 0 };

        self.pbytes(1, byte_value)
    }

    fn pbytes(&mut self, size: usize, value: usize) -> io::Result<()> {
        if self.bit_index == 0 {
            let mut idx = size - 1;
            while idx > 0 {
                self.buffer[idx] = (value >> Self::VALUES[idx]) as u8;
                idx -= 1;
            }

            self.writer_pos += size;
            return Ok(());
        }

        self.pbits(value << Self::BITS_PER_BYTE, value)
    }

    pub fn pbits(&mut self, size: usize, value: usize) -> io::Result<()> {
        // Value being written will exceed the space of a single byte so
        // we need to determine the bits leftover to handle.
        if self.bit_index + size > Self::BITS_PER_BYTE {
            let remainder_bits = size + self.bit_index - Self::BITS_PER_BYTE;
            let remainder_bytes =
                (value as i32 & Self::VALUES[remainder_bits - Self::BITS_PER_BYTE]) as usize;

            self.bit_index = 0;
            self.buffer[self.writer_pos] |= (value >> remainder_bits) as u8;
            self.writer_pos += 1;

            return self.pbits(remainder_bytes << Self::BITS_PER_BYTE, remainder_bits);
        }

        // Otherwise write a value that will fit within a byte.
        let offset = Self::BITS_PER_BYTE - (self.bit_index) - size;
        self.buffer[self.writer_pos] |= (value << offset) as u8;
        self.bit_index += size;

        // Reset bit index to start of next byte.
        if self.bit_index == Self::BITS_PER_BYTE {
            self.bit_index = 0;
            self.writer_pos += 1;
        }

        Ok(())
    }
}
