use std::cmp::min;
use std::io;

/// A specialized buffer that allows for reading and writing data at bit-level granularity
pub struct PacketBit {
    /// Current write byte position in buffer
    writer_byte_pos: usize,
    /// Current write bit position within the current byte (0-7)
    writer_bit_pos: usize,
    /// Current read byte position in buffer
    reader_byte_pos: usize,
    /// Current read bit position within the current byte (0-7)
    reader_bit_pos: usize,
    /// Underlying buffer for data storage
    buffer: Box<[u8]>,
}

impl PacketBit {
    /// Bit masks for efficient bit operations (2^n - 1)
    const BIT_MASKS: [u32; 33] = [
        0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff, 0x1ff, 0x3ff, 0x7ff, 0xfff, 0x1fff,
        0x3fff, 0x7fff, 0xffff, 0x1ffff, 0x3ffff, 0x7ffff, 0xfffff, 0x1fffff, 0x3fffff, 0x7fffff,
        0xffffff, 0x1ffffff, 0x3ffffff, 0x7ffffff, 0xfffffff, 0x1fffffff, 0x3fffffff, 0x7fffffff,
        0xffffffff,
    ];

    /// Number of bits in a byte
    const BITS_PER_BYTE: usize = 8;

    /// Create a new empty PacketBit with default capacity
    pub fn new() -> Self {
        Self::with_capacity(32)
    }

    /// Create a new PacketBit with specified capacity in bytes
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            writer_byte_pos: 0,
            writer_bit_pos: 0,
            reader_byte_pos: 0,
            reader_bit_pos: 0,
            buffer: vec![0u8; capacity].into_boxed_slice(),
        }
    }

    /// Create a PacketBit from existing data
    pub fn from_bytes(data: &[u8]) -> Self {
        let mut buffer = vec![0u8; data.len()].into_boxed_slice();
        buffer.copy_from_slice(data);

        Self {
            writer_byte_pos: data.len(),
            writer_bit_pos: 0,
            reader_byte_pos: 0,
            reader_bit_pos: 0,
            buffer,
        }
    }

    /// Write a boolean value (1 bit)
    pub fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.write_bits(if value { 1 } else { 0 }, 1)
    }

    /// Write an unsigned byte value (8 bits)
    pub fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write_bits(value as u32, 8)
    }

    /// Write an unsigned 16-bit value
    pub fn write_u16(&mut self, value: u16) -> io::Result<()> {
        self.write_bits(value as u32, 16)
    }

    /// Write an unsigned 32-bit value
    pub fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.write_bits(value, 32)
    }

    /// Write arbitrary number of bits (up to 32) from a value
    pub fn write_bits(&mut self, value: u32, num_bits: usize) -> io::Result<()> {
        if num_bits == 0 || num_bits > 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid number of bits: {}", num_bits),
            ));
        }

        // Ensure we have enough capacity
        self.ensure_capacity(self.writer_byte_pos + ((self.writer_bit_pos + num_bits + 7) / 8))?;

        // Handle the case where we're not byte-aligned
        let mut bits_remaining = num_bits;
        let mut value_remaining = value & Self::BIT_MASKS[num_bits]; // Mask to ensure we only use the specified bits

        while bits_remaining > 0 {
            // How many bits can we write into the current byte
            let bits_available = Self::BITS_PER_BYTE - self.writer_bit_pos;
            let bits_to_write = min(bits_remaining, bits_available);

            // Extract the bits we want to write into this byte
            let shift = bits_remaining - bits_to_write;
            let bits_for_this_byte = (value_remaining >> shift) & Self::BIT_MASKS[bits_to_write];

            // Position these bits correctly in the byte
            let byte_shift = bits_available - bits_to_write;
            self.buffer[self.writer_byte_pos] |= (bits_for_this_byte << byte_shift) as u8;

            // Update our position
            self.writer_bit_pos += bits_to_write;
            if self.writer_bit_pos >= Self::BITS_PER_BYTE {
                self.writer_byte_pos += 1;
                self.writer_bit_pos = 0;
            }

            // Update remaining bits
            bits_remaining -= bits_to_write;
            value_remaining &= Self::BIT_MASKS[bits_remaining];
        }

        Ok(())
    }

    /// Read a boolean value (1 bit)
    pub fn read_bool(&mut self) -> io::Result<bool> {
        self.read_bits(1).map(|value| value == 1)
    }

    /// Read an unsigned byte value (8 bits)
    pub fn read_u8(&mut self) -> io::Result<u8> {
        self.read_bits(8).map(|value| value as u8)
    }

    /// Read an unsigned 16-bit value
    pub fn read_u16(&mut self) -> io::Result<u16> {
        self.read_bits(16).map(|value| value as u16)
    }

    /// Read an unsigned 32-bit value
    pub fn read_u32(&mut self) -> io::Result<u32> {
        self.read_bits(32)
    }

    /// Read arbitrary number of bits (up to 32) into a u32 value
    pub fn read_bits(&mut self, num_bits: usize) -> io::Result<u32> {
        if num_bits == 0 || num_bits > 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid number of bits: {}", num_bits),
            ));
        }

        // Check if we have enough data
        let required_bytes = (self.reader_bit_pos + num_bits + 7) / 8;
        if self.reader_byte_pos + required_bytes > self.writer_byte_pos + (self.writer_bit_pos > 0) as usize {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough data in buffer",
            ));
        }

        let mut result: u32 = 0;
        let mut bits_remaining = num_bits;

        while bits_remaining > 0 {
            // How many bits can we read from the current byte
            let bits_available = Self::BITS_PER_BYTE - self.reader_bit_pos;
            let bits_to_read = min(bits_remaining, bits_available);

            // Extract the bits from this byte
            let byte_shift = bits_available - bits_to_read;
            let mask = Self::BIT_MASKS[bits_to_read] as u8;
            let bits_from_this_byte = (self.buffer[self.reader_byte_pos] >> byte_shift) & mask;

            // Position these bits correctly in the result
            let result_shift = bits_remaining - bits_to_read;
            result |= (bits_from_this_byte as u32) << result_shift;

            // Update our position
            self.reader_bit_pos += bits_to_read;
            if self.reader_bit_pos >= Self::BITS_PER_BYTE {
                self.reader_byte_pos += 1;
                self.reader_bit_pos = 0;
            }

            // Update remaining bits
            bits_remaining -= bits_to_read;
        }

        Ok(result)
    }

    /// Align writer to the next byte boundary
    pub fn align_writer(&mut self) {
        if self.writer_bit_pos > 0 {
            self.writer_byte_pos += 1;
            self.writer_bit_pos = 0;
        }
    }

    /// Align reader to the next byte boundary
    pub fn align_reader(&mut self) {
        if self.reader_bit_pos > 0 {
            self.reader_byte_pos += 1;
            self.reader_bit_pos = 0;
        }
    }

    /// Reset the reader position to the beginning of the buffer
    pub fn reset_reader(&mut self) {
        self.reader_byte_pos = 0;
        self.reader_bit_pos = 0;
    }

    /// Ensure the buffer has enough capacity
    fn ensure_capacity(&mut self, required_capacity: usize) -> io::Result<()> {
        if required_capacity <= self.buffer.len() {
            return Ok(());
        }

        // Calculate new capacity (double current size until sufficient)
        let mut new_capacity = self.buffer.len();
        while new_capacity < required_capacity {
            new_capacity *= 2;
        }

        // Create new buffer and copy data
        let mut new_buffer = vec![0u8; new_capacity].into_boxed_slice();
        new_buffer[..self.buffer.len()].copy_from_slice(&self.buffer);
        self.buffer = new_buffer;

        Ok(())
    }

    pub fn as_slice(&self) -> &[u8] {
        let end = self.writer_byte_pos + (self.writer_bit_pos > 0) as usize;
        &self.buffer[..end]
    }

    pub fn write_to_packet(&self, packet: &mut dyn PacketWriter) -> Result<(), PacketError> {
        packet.put_slice(self.as_slice())?;
        Ok(())
    }

    pub fn writer_byte_position(&self) -> usize {
        self.writer_byte_pos
    }

    pub fn writer_bit_position(&self) -> usize {
        self.writer_bit_pos
    }

    pub fn reader_byte_position(&self) -> usize {
        self.reader_byte_pos
    }

    pub fn reader_bit_position(&self) -> usize {
        self.reader_bit_pos
    }

    pub fn bits_written(&self) -> usize {
        (self.writer_byte_pos * Self::BITS_PER_BYTE) + self.writer_bit_pos
    }

    pub fn bits_read(&self) -> usize {
        (self.reader_byte_pos * Self::BITS_PER_BYTE) + self.reader_bit_pos
    }

    pub fn bytes_written(&self) -> usize {
        self.writer_byte_pos + (self.writer_bit_pos > 0) as usize
    }
}

pub trait PacketWriter {
    fn put_slice(&mut self, data: &[u8]) -> Result<(), PacketError>;
}


