pub mod packet;

macro_rules! g {
    ($buf:ident, $byte_ty:ty, $conversion:expr) => {{
        const SIZE: usize = std::mem::size_of::<$byte_ty>();
        let limit = $buf.write_pos;
        let pos = $buf.read_pos;
        if pos + SIZE > limit {
            return error(IoError::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "expected pos + size_in_bytes < limit. (pos: {}, size_in_bytes: {}, limit: {})",
                    pos, SIZE, limit
                ),
            ));
        }

        let slice = unsafe { *($buf.buf[pos..pos + SIZE].as_ptr() as *const [_; SIZE]) };
        $buf.read_pos += SIZE;
        Ok($conversion(slice))
    }};
}

macro_rules! p {
    ($this:tt, $size:literal, $value:tt) => {{
        let pos = $this.write_pos;
        let slice_len = $value.len();
        let buf_len = $this.buf.len();
        if pos + slice_len >= buf_len {
            $this.buf.resize((slice_len + buf_len) * 2, 0u8);
        }

        $this.buf.deref_mut()[pos..pos + slice_len].copy_from_slice($value);
        $this.write_pos += slice_len;
    }};
}

#[cfg(test)]
mod test {
    use crate::packet::bytes::Packet;
    use crate::packet::error::PacketError;
    use crate::packet::bits::{BitReader, BitWriter};

    #[test]
    fn test_read_string() -> Result<(), PacketError> {
        let str = "hello";
        let mut packet = Packet::new(str.len() + 1);
        // Write the str into the packet.
        packet.pjstr(&str);

        // Set the cursor back to zero to prepare the read.
        packet.set_pos(0)?;

        // Read a null-terminated string from the packet.
        println!("pos1 {}", packet.get_pos());
        let value = packet.gjstr()?;
        println!("pos2 {}", packet.get_pos());
        println!("len {}, {:?}", packet.len(), value);
        assert_eq!(value, "hello");
        Ok(())
    }

    #[test]
    fn test_read_smart_int() {
        let mut packet = Packet::new(4);
        packet.psmart_u32(20);
        packet.set_pos(0).unwrap();
        assert_eq!(20, packet.gsmart_u32().unwrap());
    }

    #[test]
    fn test_alt1_read() {
        let mut packet = Packet::new(1);
        packet.p2_alt2(10);
        packet.set_pos(0);
        println!("{:?}", packet);
        let value = packet.g2_alt2().unwrap();
        println!("{:?}", value);
        // bits.writ(&mut packet);
    }

    #[test]
    fn test_bit_write_read() {
        // Write some data
        let mut buffer = Packet::new(100);
        buffer.p1(4);
        buffer.p2(100);
        {
            let mut writer = BitWriter::from(&mut buffer);
            writer.write_bits(2, 3).unwrap();
            writer.write_bits(100, 8).unwrap(); // crossing byte boundary
            writer.write_bits(0, 1).unwrap();
            writer.write_bits(2000, 16).unwrap();
        }

        println!("{:?}", buffer);
        buffer.set_pos(0);
        println!("{:?}", buffer.g1());
        println!("{:?}", buffer.g2());
        {
            let mut reader = BitReader::from(&buffer);
            assert_eq!(reader.read_bits(3).unwrap(), 2);
            assert_eq!(reader.read_bits(8).unwrap(), 100);
            assert_eq!(reader.read_bits(1).unwrap(), 0);
            assert_eq!(reader.read_bits(16).unwrap(), 2000);
        }
    }
}