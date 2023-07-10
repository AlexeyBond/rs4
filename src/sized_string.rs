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

    pub fn validate_content(&self, safe_address_range: AddressRange) -> Result<(), MemoryAccessError> {
        let length = self.read_length() as u16;

        self.memory.validate_access(
            self.address.wrapping_add(1)..=(self.address.wrapping_add(1 + length)),
            safe_address_range,
        )?;

        Ok(())
    }

    pub unsafe fn unsafe_new(memory: &Mem, address: Address) -> ReadableSizedString {
        ReadableSizedString { memory, address }
    }

    pub fn as_bytes(&self) -> &'m [u8] {
        let length = self.read_length() as usize;

        return self.memory.slice((self.address as usize + 1)..(self.address as usize + 1 + length));
    }
}

#[cfg(test)]
mod test {
    use crate::mem::Mem;
    use crate::sized_string::ReadableSizedString;

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
        let start_address = *mem.address_range().end() - 255;

        mem.write_u8(start_address, 255);

        assert!(ReadableSizedString::new(&mem, start_address, mem.address_range()).is_err())
    }
}
