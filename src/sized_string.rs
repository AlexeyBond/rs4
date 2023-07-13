use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};

pub struct ReadableSizedString<'m> {
    memory: &'m Mem,
    address: Address,
}

impl<'m> ReadableSizedString<'m> {
    pub fn new(memory: &'m Mem, address: Address, safe_address_range: AddressRange) -> Result<ReadableSizedString, MemoryAccessError> {
        memory.validate_access(
            address..=address,
            safe_address_range.clone(),
        )?;

        let sz_str = unsafe { ReadableSizedString::unsafe_new(memory, address) };

        sz_str.validate_content(safe_address_range)?;

        Ok(sz_str)
    }

    pub fn read_length(&self) -> u8 {
        self.memory.read_u8(self.address)
    }

    pub fn content_address(&self) -> Address {
        self.address.wrapping_add(1)
    }

    pub fn validate_content(&self, safe_address_range: AddressRange) -> Result<(), MemoryAccessError> {
        let length = self.read_length() as u16;
        let content_address = self.content_address();

        if length > 0 {
            self.memory.validate_access(
                content_address..=(content_address.wrapping_add(length - 1)),
                safe_address_range,
            )?;
        }

        Ok(())
    }

    pub fn full_range(&self) -> AddressRange {
        self.address..=(self.address.wrapping_add(1).wrapping_add(self.read_length() as u16))
    }

    pub unsafe fn unsafe_new(memory: &Mem, address: Address) -> ReadableSizedString {
        ReadableSizedString { memory, address }
    }

    pub fn as_bytes(&self) -> &'m [u8] {
        let length = self.read_length() as usize;

        return self.memory.slice((self.address as usize + 1)..(self.address as usize + 1 + length));
    }
}

pub struct SizedStringWriter<'m> {
    memory: &'m mut Mem,
    address: Address,
    len: u8,
    max_len: u8,
}

impl<'m> SizedStringWriter<'m> {
    fn new(memory: &'m mut Mem, address: Address, max_len: u8, safe_range: AddressRange) -> Result<SizedStringWriter, MemoryAccessError> {
        memory.validate_access(
            address..=(address.wrapping_add(max_len as u16)),
            safe_range,
        )?;

        Ok(SizedStringWriter {
            memory,
            address,
            len: 0,
            max_len,
        })
    }

    fn writeable_range(&self) -> AddressRange {
        self.address..=(self.address.wrapping_add(self.max_len as u16))
    }

    fn append_u8(&mut self, value: u8) -> Result<(), MemoryAccessError> {
        if self.len >= self.max_len {
            return Err(MemoryAccessError {
                access_range: self.address..=(self.address.wrapping_add(self.len as u16).wrapping_add(1)),
                segment: self.writeable_range(),
            });
        }

        self.len += 1;
        self.memory.write_u8(self.address.wrapping_add(self.len as u16), value);

        Ok(())
    }

    fn append_slice(&mut self, value: &[u8]) -> Result<(), MemoryAccessError> {
        self.memory.validate_access(
            self.address..=(self.address.wrapping_add(self.len as u16).wrapping_add(value.len() as u16)),
            self.writeable_range(),
        )?;

        self.memory.address_slice_mut(self.address + 1 + self.len as u16, value.len()).copy_from_slice(value);

        self.len += value.len() as u8;

        Ok(())
    }

    fn finish(self) -> ReadableSizedString<'m> {
        self.memory.write_u8(self.address, self.len);

        unsafe { ReadableSizedString::unsafe_new(self.memory, self.address) }
    }
}

#[cfg(test)]
mod test {
    use crate::mem::Mem;
    use crate::sized_string::{ReadableSizedString, SizedStringWriter};

    #[test]
    fn test_read_sized_string() {
        let mut mem = Mem::default();

        mem.write_u8(12345, 3);
        mem.write_u8(12346, 'b' as u8);
        mem.write_u8(12347, 'a' as u8);
        mem.write_u8(12348, 'r' as u8);

        assert_eq!(ReadableSizedString::new(&mem, 12345, mem.address_range()).unwrap().as_bytes(), "bar".as_bytes());
    }

    #[test]
    fn test_bad_string() {
        let mut mem = Mem::default();
        let start_address = *mem.address_range().end() - 254;

        mem.write_u8(start_address, 255);

        assert!(ReadableSizedString::new(&mem, start_address, mem.address_range()).is_err())
    }

    #[test]
    fn test_longest_string() {
        let mut mem = Mem::default();
        let start_address = *mem.address_range().end() - 255;

        mem.write_u8(start_address, 255);

        assert_eq!(
            ReadableSizedString::new(&mem, start_address, mem.address_range())
                .unwrap()
                .as_bytes()
                .len(),
            255
        );
    }

    #[test]
    fn test_write_chars() {
        let mut mem = Mem::default();
        let safe_range = mem.address_range();

        let mut writer = SizedStringWriter::new(&mut mem, 123, 255, safe_range).unwrap();

        writer.append_u8(b'F').unwrap();
        writer.append_u8(b'O').unwrap();
        writer.append_u8(b'0').unwrap();
        writer.append_u8(b'B').unwrap();
        writer.append_u8(b'A').unwrap();
        writer.append_u8(b'R').unwrap();

        assert_eq!(
            writer.finish().as_bytes(),
            b"FO0BAR"
        )
    }

    #[test]
    fn test_write_overflow() {
        let mut mem = Mem::default();
        let safe_range = mem.address_range();

        let mut writer = SizedStringWriter::new(&mut mem, 123, 255, safe_range).unwrap();

        for _ in 0..255 {
            writer.append_u8(b'A').unwrap();
        }

        assert!(writer.append_u8(b'B').is_err())
    }

    #[test]
    fn test_write_max_length() {
        let mut mem = Mem::default();
        let safe_range = mem.address_range();

        let mut writer = SizedStringWriter::new(&mut mem, 123, 2, safe_range).unwrap();

        writer.append_slice(b"AA").unwrap();

        assert!(writer.append_u8(b'B').is_err())
    }

    #[test]
    fn test_write_string() {
        let mut mem = Mem::default();
        let safe_range = mem.address_range();

        let mut writer = SizedStringWriter::new(&mut mem, 123, 255, safe_range).unwrap();

        writer.append_slice(b"Hello ").unwrap();
        writer.append_slice(b"World!").unwrap();

        assert_eq!(
            writer.finish().as_bytes(),
            b"Hello World!"
        )
    }
}
