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
    use crate::packet::bits::PacketBit;
    use crate::packet::bytes::Packet;
    use crate::packet::error::PacketError;

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
    fn test_read_bits() {
        let mut packet = Packet::new(5);
        let mut bits = PacketBit::new();
        bits.pbits(30, 445);
        println!("{}", bits.writer_byte_position());
        bits.pbits(5, 6);
        println!("{}", bits.writer_byte_position());
        bits.pbits(14, 36);
        println!("{}", bits.writer_byte_position());
        let value1 = bits.gbits(30).unwrap();
        println!("{}", bits.reader_byte_position());
        let value2 = bits.gbits(5).unwrap();
        println!("{}", bits.reader_byte_position());
        let value3 = bits.gbits(14).unwrap();
        println!("{}", bits.reader_byte_position());

        // bits.writ(&mut packet);
        println!("{:?}", value1);
        println!("{:?}", value2);
        println!("{:?}", value3);
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
}