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

    #[inline]
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
        let total_bits_in_buffer = self.buffer.len() * 8;
        let current_total_bit_pos = self.byte_pos * 8 + self.bit_pos;

        if current_total_bit_pos + bit_count > total_bits_in_buffer {
            return Err(PacketError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected eof while skipping bits. (bit_count: {}, total_bits_in_buffer: {})",
            )));
        }

        let new_total_bit_pos = current_total_bit_pos + bit_count;
        self.byte_pos = new_total_bit_pos / 8;
        self.bit_pos = new_total_bit_pos % 8;

        Ok(())
    }
}

#[derive(Debug)]
pub struct BitWriter<'a> {
    packet: &'a mut Packet,
    bit_pos: usize,
}

impl<'a> BitWriter<'a> {
    pub fn new(buffer: &'a mut Packet) -> Self {
        Self {
            packet: buffer,
            bit_pos: 0,
        }
    }

    pub fn new_at_position(buffer: &'a mut Packet, byte_pos: usize) -> Self {
        Self {
            packet: buffer,
            bit_pos: byte_pos,
        }
    }

    #[inline]
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

        let required_len = self.packet.pos + (self.bit_pos + bit_count + 7) / 8;
        if required_len > self.packet.bytes.len() {
            self.packet.bytes.resize(required_len, 0);
        }

        let max_value = if bit_count == 32 { u32::MAX } else { (1 << bit_count) - 1 };
        let masked_value = value & max_value;

        let mut bits_remaining = bit_count;

        while bits_remaining > 0 {
            let bits_available_in_byte = 8 - self.bit_pos;
            let bits_to_write = std::cmp::min(bits_available_in_byte, bits_remaining);

            let value_shift = bits_remaining - bits_to_write;
            let bits_from_value = (masked_value >> value_shift) & BIT_MASKS[bits_to_write];

            let clear_mask_shift = bits_available_in_byte - bits_to_write;
            let clear_mask = !((BIT_MASKS[bits_to_write] as u8) << clear_mask_shift);

            self.packet.bytes[self.packet.pos] &= clear_mask;

            let set_mask = (bits_from_value as u8) << clear_mask_shift;
            self.packet.bytes[self.packet.pos] |= set_mask;

            self.bit_pos += bits_to_write;
            bits_remaining -= bits_to_write;

            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.packet.pos += 1;
            }
        }

        Ok(())
    }


    pub fn get_bits_used(&self) -> usize {
        self.bit_pos
    }

}

impl<'a> Drop for BitWriter<'a> {
    fn drop(&mut self) {
        if self.bit_pos > 0 {
            self.packet.pos += 1;
        }

        if self.packet.pos > self.packet.len {
            self.packet.len = self.packet.pos;
        }
    }
}

impl<'a> From<&'a mut Packet> for BitWriter<'a> {
    fn from(value: &'a mut Packet) -> Self {
        Self {
            packet: value,
            bit_pos: 0,
        }
    }
}

impl<'a> From<&'a Packet> for BitReader<'a> {
    fn from(value: &'a Packet) -> Self {
        Self::new(value.as_ref())
    }
}