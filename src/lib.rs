pub mod packet;
#[cfg(test)]
mod test {
    use crate::packet::bits::{BitReader, BitWriter};
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
    fn test_alt1_read() {
        let mut packet = Packet::new(2);
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

    #[cfg(feature = "macros")]
    #[test]
    fn test_macro() {
        #[derive(Debug, Protocol)]
        pub enum ClientProt {
            #[packet(opcode = 69, size = 0)]
            MapBuildComplete,

            #[packet(opcode = 77, size = 6)]
            EventMouseClick,

            // Now works correctly with negative numbers and combined attributes
            #[packet(opcode = 1, size = -1)]
            EventKeyboard,

            #[packet(opcode = 72, size = 4)]
            DetectModifiedClient,
        }

        assert_eq!(ClientProt::MapBuildComplete.opcode(), 69, "Must be equal to 1");
        assert_eq!(ClientProt::EventMouseClick.opcode(), 77, "Must be equal to 2");
        assert_eq!(ClientProt::DetectModifiedClient.opcode(), 72, "Must be equal to 3");
    }
}
