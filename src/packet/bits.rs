const BIT_MASKS: [u32; 33] = [
    0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff,
    0x3fff, 0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff,
    0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff,
    0xffffffff,
];

use crate::packet::bytes::Packet;
use crate::packet::error::PacketError;
use std::io;

#[derive(Debug)]
pub struct BitReader<'a> {
    buffer: &'a [u8],
    byte_pos: usize,
    bit_pos: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self {
            buffer,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    pub fn new_at_position(buffer: &'a [u8], byte_pos: usize) -> Self {
        Self {
            buffer,
            byte_pos,
            bit_pos: 0,
        }
    }

    pub fn read_bits(&mut self, bit_count: usize) -> Result<usize, PacketError> {
        if bit_count > 32 {
            return Err(PacketError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Bit count cannot exceed 32",
            )));
        }

        if bit_count == 0 {
            return Ok(0);
        }

        let mut result = 0;
        let mut bits_remaining = bit_count;

        while bits_remaining > 0 {
            if self.byte_pos >= self.buffer.len() {
                return Err(PacketError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "End of buffer reached",
                )));
            }

            let bits_available_in_byte = 8 - self.bit_pos;
            let bits_to_read = std::cmp::min(bits_available_in_byte, bits_remaining);

            let current_byte = self.buffer[self.byte_pos];

            let shift = bits_available_in_byte - bits_to_read;
            let mask = BIT_MASKS[bits_to_read] as usize;

            let bits = ((current_byte >> shift) & mask as u8) as usize;
            result = (result << bits_to_read) | bits;

            self.bit_pos += bits_to_read;
            bits_remaining -= bits_to_read;

            if self.bit_pos == 8 {
                self.byte_pos += 1;
                self.bit_pos = 0;
            }
        }

        Ok(result)
    }

    pub fn get_bit_position(&self) -> usize {
        self.byte_pos * 8 + self.bit_pos
    }

    pub fn has_bits_available(&self, bit_count: usize) -> bool {
        let total_bits_in_buffer = self.buffer.len() * 8;
        let bits_consumed = self.byte_pos * 8 + self.bit_pos;

        total_bits_in_buffer - bits_consumed >= bit_count
    }

    pub fn get_bits_used(&self) -> usize {
        self.bit_pos
    }

    pub fn get_buffer(&self) -> &[u8] {
        self.buffer
    }

    pub fn skip_bits(&mut self, bit_count: usize) -> Result<(), PacketError> {
        let mut bits_to_skip = bit_count;

        while bits_to_skip > 0 {
            let bits_remaining_in_byte = 8 - self.bit_pos;
            let bits_skip_now = std::cmp::min(bits_remaining_in_byte, bits_to_skip);

            self.bit_pos += bits_skip_now;

            if self.bit_pos == 8 {
                self.byte_pos += 1;
                self.bit_pos = 0;

                if self.byte_pos > self.buffer.len() {
                    return Err(PacketError::Io(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "End of buffer reached while skipping",
                    )));
                }
            }

            bits_to_skip -= bits_skip_now;
        }

        Ok(())
    }
}

impl<'a> From<&'a Packet> for BitReader<'a> {
    fn from(value: &'a Packet) -> Self {
        Self::new_at_position(value, value.get_pos())
    }
}

#[derive(Debug)]
pub struct BitWriter<'a> {
    buffer: &'a mut Vec<u8>,
    byte_pos: usize,
    bit_pos: usize,
}

impl<'a> BitWriter<'a> {
    pub fn new(buffer: &'a mut Vec<u8>) -> Self {
        let byte_pos = buffer.len();
        Self {
            buffer,
            byte_pos,
            bit_pos: 0,
        }
    }

    pub fn new_at_position(buffer: &'a mut Vec<u8>, byte_pos: usize) -> Self {
        Self {
            buffer,
            byte_pos,
            bit_pos: 0,
        }
    }

    pub fn write_bits(&mut self, value: u32, bit_count: usize) -> Result<(), PacketError> {
        if bit_count > 32 {
            return Err(PacketError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Bit count cannot exceed 32",
            )));
        }

        if bit_count == 0 {
            return Ok(());
        }

        let max_value = if bit_count == 32 { 0xFFFFFFFF } else { (1 << bit_count) - 1 };
        let masked_value = value & max_value;

        let mut bits_remaining = bit_count;
        let bits_value = masked_value;

        while bits_remaining > 0 {
            if self.byte_pos >= self.buffer.len() {
                self.buffer.push(0);
            }

            let bits_available_in_byte = 8 - self.bit_pos;
            let bits_to_write = std::cmp::min(bits_available_in_byte, bits_remaining);

            let shift = bits_remaining - bits_to_write;
            let mask = BIT_MASKS[bits_to_write];
            let bits = (bits_value >> shift) & mask;

            let bit_shift = bits_available_in_byte - bits_to_write;
            let byte_mask = (bits as u8) << bit_shift;

            self.buffer[self.byte_pos] |= byte_mask;

            self.bit_pos += bits_to_write;
            bits_remaining -= bits_to_write;

            if self.bit_pos == 8 {
                self.byte_pos += 1;
                self.bit_pos = 0;
            }
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        if self.bit_pos > 0 {
            self.byte_pos += 1;
            self.bit_pos = 0;
        }
    }

    pub fn get_bits_used(&self) -> usize {
        self.bit_pos
    }

    pub fn get_byte_pos(&self) -> usize {
        self.byte_pos
    }

    pub fn get_buffer(&self) -> &Vec<u8> {
        self.buffer
    }
}

impl<'a> From<&'a mut Packet> for BitWriter<'a> {
    fn from(value: &'a mut Packet) -> Self {
        let pos = value.get_pos();
        Self::new_at_position(value.get_inner_mut(), pos)
    }
}