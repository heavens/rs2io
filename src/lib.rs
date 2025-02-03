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