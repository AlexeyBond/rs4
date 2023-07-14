use std::io;
use std::ops::{Range, RangeInclusive};

const MEM_SIZE: usize = (u16::MAX as usize) + 1;

/// A piece of memory that allows access to it's random fragments of different sizes.
#[derive(Clone)]
pub struct Mem {
    content: [u8; MEM_SIZE],
}

pub type Address = u16;

pub type AddressRange = RangeInclusive<Address>;

#[derive(Debug)]
pub struct MemoryAccessError {
    pub access_range: AddressRange,
    pub segment: AddressRange,
}

impl Default for Mem {
    fn default() -> Self {
        return Mem {
            content: [0; MEM_SIZE]
        };
    }
}

impl Mem {
    pub fn address_range(&self) -> AddressRange {
        0..=Address::MAX
    }

    pub fn validate_access(
        &self,
        address_range: AddressRange,
        segment: AddressRange,
    ) -> Result<(), MemoryAccessError> {
        if *address_range.start() > *address_range.end() || *address_range.start() < *segment.start() || *address_range.end() > *segment.end() {
            return Err(MemoryAccessError {
                access_range: address_range,
                segment,
            });
        }

        return Ok(());
    }

    pub fn read_u8(&self, offset: Address) -> u8 {
        self.content[offset as usize]
    }

    pub fn write_u8(&mut self, offset: Address, value: u8) {
        self.content[offset as usize] = value
    }

    pub unsafe fn read_u16(&self, offset: Address) -> u16 {
        (self.content.as_ptr().offset(offset as isize) as *const u16).read()
    }

    pub unsafe fn write_u16(&mut self, offset: Address, value: u16) {
        (self.content.as_mut_ptr().offset(offset as isize) as *mut u16).write(value)
    }

    pub unsafe fn read_u32(&self, offset: Address) -> u32 {
        (self.content.as_ptr().offset(offset as isize) as *const u32).read()
    }

    pub unsafe fn write_u32(&mut self, offset: Address, value: u32) {
        (self.content.as_mut_ptr().offset(offset as isize) as *mut u32).write(value)
    }

    pub fn slice(&self, range: Range<usize>) -> &[u8] {
        return &self.content[range];
    }

    pub fn address_slice(&self, start: Address, length: usize) -> &[u8] {
        return self.slice((start as usize)..((start as usize) + length));
    }

    pub fn slice_mut(&mut self, range: Range<usize>) -> &mut [u8] {
        return &mut self.content[range];
    }

    pub fn address_slice_mut(&mut self, start: Address, length: usize) -> &mut [u8] {
        return self.slice_mut((start as usize)..((start as usize) + length));
    }

    pub fn dump_to(&self, dst: &mut impl io::Write) -> io::Result<()> {
        dst.write_all(&self.content)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rw_u8() {
        let mut mem: Mem = Mem::default();

        mem.write_u8(123, 144);

        assert_eq!(
            mem.read_u8(123),
            144,
        );
    }

    #[test]
    fn test_rw_u16() {
        let mut mem: Mem = Mem::default();

        unsafe {
            mem.write_u16(54345, 0xabcd);
        };

        assert_eq!(
            unsafe {
                mem.read_u16(54345)
            },
            0xabcd,
        );
    }

    #[test]
    fn test_rw_u32() {
        let mut mem: Mem = Mem::default();

        unsafe {
            mem.write_u32(12345, 0x1234abcd);
        };

        assert_eq!(
            unsafe {
                mem.read_u32(12345)
            },
            0x1234abcd,
        );
    }

    #[test]
    fn test_min_max_addresses() {
        let mem: Mem = Mem::default();

        assert_eq!(mem.content[*mem.address_range().start() as usize], 0);
        assert_eq!(mem.content[*mem.address_range().end() as usize], 0);
    }
}
