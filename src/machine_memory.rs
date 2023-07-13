use int_enum::IntEnum;

use crate::input::{Input, InputError};
use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};
use crate::opcodes::OpCode;
use crate::readable_article::ReadableArticle;
use crate::sized_string::ReadableSizedString;

#[derive(Copy, Clone)]
pub struct MemoryLayoutConfig {
    pub max_call_stack_depth: u16,
}

impl Default for MemoryLayoutConfig {
    fn default() -> Self {
        MemoryLayoutConfig {
            max_call_stack_depth: 128,
        }
    }
}

#[repr(u16)]
#[derive(Clone, Copy, PartialEq, Debug, IntEnum)]
pub enum ReservedAddresses {
    BaseVar = 10,

    WordBuffer = 256,
    ScratchBuffer = 512,

    Max = 769, // 2 * 256 bytes for buffers + 256 bytes for built-in variables + 1 to get address of the last byte
}

/// A virtual machine's memory along with "registers" representing current layout and usage of the
/// memory.
#[derive(Clone)]
pub struct MachineMemory {
    /// Address of the next byte that will be used by dictionary.
    pub dict_ptr: Address,

    pub last_article_ptr: Option<Address>,

    /// Address of the last pushed word on data stack
    /// or address immediately after the data stack if data stack is empty.
    pub data_stack_ptr: Address,

    /// Lowest address available for call stack.
    stacks_border: Address,

    /// Address of the most recent word on call stack
    /// or address immediately after call stack if call stack is empty.
    pub call_stack_ptr: Address,

    /// Lowest address reserved for built-in variables.
    reserved_space_start: Address,

    pub raw_memory: Mem,
}

impl Default for MachineMemory {
    fn default() -> Self {
        MachineMemory::new(Mem::default(), MemoryLayoutConfig::default())
    }
}

impl MachineMemory {
    pub fn new(memory: Mem, config: MemoryLayoutConfig) -> MachineMemory {
        let total_range = memory.address_range();
        let reserved_space_start = (*total_range.end() - ReservedAddresses::Max.int_value()) + 1;
        let stacks_border = reserved_space_start - 2 * config.max_call_stack_depth;

        let mut mm = MachineMemory {
            last_article_ptr: None,
            dict_ptr: *total_range.start(),
            reserved_space_start,
            call_stack_ptr: reserved_space_start,
            stacks_border,
            data_stack_ptr: stacks_border,

            raw_memory: memory,
        };

        mm.reset_builtin_vars();

        mm
    }

    fn reset_builtin_vars(&mut self) {
        unsafe {
            self.raw_memory.write_u16(
                self.get_reserved_address(ReservedAddresses::BaseVar),
                10,
            );
        }
    }

    /// Reset mutable pointers and some reserved variables to initial values.
    pub fn reset(&mut self) {
        self.last_article_ptr = None;
        self.dict_ptr = *self.raw_memory.address_range().start();
        self.call_stack_ptr = self.reserved_space_start;
        self.data_stack_ptr = self.stacks_border;

        self.reset_builtin_vars()
    }

    /// Current depth of call stack in words.
    pub fn call_stack_depth(&self) -> u16 {
        self.reserved_space_start.wrapping_sub(self.call_stack_ptr) >> 1
    }

    /// Current depth of data stack in words.
    pub fn data_stack_depth(&self) -> u16 {
        self.stacks_border.wrapping_sub(self.data_stack_ptr) >> 1
    }

    /// Get address in reserved address space corresponding to given `ReservedAddress`.
    pub fn get_reserved_address(&self, address: ReservedAddresses) -> Address {
        self.reserved_space_start + address.int_value()
    }

    /// Range of addresses available for use by call stack.
    pub fn get_call_stack_segment(&self) -> AddressRange {
        self.stacks_border..=(self.reserved_space_start - 1)
    }

    /// Range of addresses currently available for use by data stack.
    ///
    /// May change with writes to dictionary.
    pub fn get_data_stack_segment(&self) -> AddressRange {
        self.dict_ptr..=(self.stacks_border - 1)
    }

    /// Range of data space addresses that are not used by dict or data stack
    pub fn get_free_data_segment(&self) -> AddressRange {
        self.dict_ptr..=(self.data_stack_ptr - 1)
    }

    /// Range of addresses currently used by dictionary.
    pub fn get_used_dict_segment(&self) -> AddressRange {
        (*self.raw_memory.address_range().start())..=(self.dict_ptr.saturating_sub(1))
    }

    fn push_u16(memory: &mut Mem, sp: &mut Address, safe_range: AddressRange, value: u16) -> Result<(), MemoryAccessError> {
        let next_sp = (*sp).wrapping_sub(2);

        memory.validate_access(
            next_sp..=next_sp.wrapping_add(1),
            safe_range,
        )?;

        unsafe { memory.write_u16(next_sp, value) };

        *sp = next_sp;

        Ok(())
    }

    fn pop_u16(memory: &mut Mem, sp: &mut Address, safe_range: AddressRange) -> Result<u16, MemoryAccessError> {
        memory.validate_access(
            (*sp)..=sp.wrapping_add(1),
            safe_range,
        )?;

        let value = unsafe { memory.read_u16(*sp) };

        *sp = sp.wrapping_add(2);

        Ok(value)
    }

    fn push_u32(memory: &mut Mem, sp: &mut Address, safe_range: AddressRange, value: u32) -> Result<(), MemoryAccessError> {
        let next_sp = (*sp).wrapping_sub(4);

        memory.validate_access(
            next_sp..=next_sp.wrapping_add(3),
            safe_range,
        )?;

        unsafe { memory.write_u32(next_sp, value) };

        *sp = next_sp;

        Ok(())
    }

    fn pop_u32(memory: &mut Mem, sp: &mut Address, safe_range: AddressRange) -> Result<u32, MemoryAccessError> {
        memory.validate_access(
            (*sp)..=sp.wrapping_add(1),
            safe_range,
        )?;

        let value = unsafe { memory.read_u32(*sp) };

        *sp = sp.wrapping_add(4);

        Ok(value)
    }

    pub fn data_push_u16(&mut self, value: u16) -> Result<(), MemoryAccessError> {
        let segment = self.get_data_stack_segment();
        MachineMemory::push_u16(&mut self.raw_memory, &mut self.data_stack_ptr, segment, value)
    }

    pub fn data_pop_u16(&mut self) -> Result<u16, MemoryAccessError> {
        let segment = self.get_data_stack_segment();
        MachineMemory::pop_u16(&mut self.raw_memory, &mut self.data_stack_ptr, segment)
    }

    pub fn data_push_u32(&mut self, value: u32) -> Result<(), MemoryAccessError> {
        let segment = self.get_data_stack_segment();
        MachineMemory::push_u32(&mut self.raw_memory, &mut self.data_stack_ptr, segment, value)
    }

    pub fn data_pop_u32(&mut self) -> Result<u32, MemoryAccessError> {
        let segment = self.get_data_stack_segment();
        MachineMemory::pop_u32(&mut self.raw_memory, &mut self.data_stack_ptr, segment)
    }

    pub fn call_push_u16(&mut self, value: u16) -> Result<(), MemoryAccessError> {
        let segment = self.get_call_stack_segment();
        MachineMemory::push_u16(&mut self.raw_memory, &mut self.call_stack_ptr, segment, value)
    }

    pub fn call_pop_u16(&mut self) -> Result<u16, MemoryAccessError> {
        let segment = self.get_call_stack_segment();
        MachineMemory::pop_u16(&mut self.raw_memory, &mut self.call_stack_ptr, segment)
    }

    pub fn dict_write_u8(&mut self, value: u8) -> Result<(), MemoryAccessError> {
        self.raw_memory.validate_access(
            self.dict_ptr..=self.dict_ptr,
            self.get_free_data_segment(),
        )?;

        self.raw_memory.write_u8(self.dict_ptr, value);
        self.dict_ptr = self.dict_ptr.wrapping_add(1);

        Ok(())
    }

    pub fn dict_write_opcode(&mut self, value: OpCode) -> Result<(), MemoryAccessError> {
        self.dict_write_u8(value.int_value())
    }

    pub fn dict_write_u16(&mut self, value: u16) -> Result<(), MemoryAccessError> {
        self.raw_memory.validate_access(
            self.dict_ptr..=(self.dict_ptr.wrapping_add(1)),
            self.get_free_data_segment(),
        )?;

        unsafe { self.raw_memory.write_u16(self.dict_ptr, value) };
        self.dict_ptr = self.dict_ptr.wrapping_add(2);

        Ok(())
    }

    pub fn dict_write_u32(&mut self, value: u32) -> Result<(), MemoryAccessError> {
        self.raw_memory.validate_access(
            self.dict_ptr..=(self.dict_ptr.wrapping_add(3)),
            self.get_free_data_segment(),
        )?;

        unsafe { self.raw_memory.write_u32(self.dict_ptr, value) };
        self.dict_ptr = self.dict_ptr.wrapping_add(4);

        Ok(())
    }

    pub fn dict_write_sized_string(&mut self, address: Address) -> Result<(), MemoryAccessError> {
        let s = ReadableSizedString::new(&self.raw_memory, address, self.raw_memory.address_range())?;
        let length = s.read_length();
        let content_address = s.content_address();

        self.raw_memory.validate_access(
            self.dict_ptr..=(self.dict_ptr.wrapping_add(1).wrapping_add(length as u16)),
            self.get_free_data_segment(),
        )?;

        self.raw_memory.write_u8(self.dict_ptr, length);

        for i in 0..(length as u16) {
            self.raw_memory.write_u8(self.dict_ptr.wrapping_add(1).wrapping_add(i), self.raw_memory.read_u8(content_address.wrapping_add(i)));
        }

        self.dict_ptr = self.dict_ptr.wrapping_add(1).wrapping_add(length as u16);

        Ok(())
    }

    pub fn lookup_article(&self, name: &[u8]) -> Result<Option<ReadableArticle>, MemoryAccessError> {
        let mut current_article = match self.last_article_ptr {
            None => { return Ok(None); }
            Some(addr) => ReadableArticle::new(&self.raw_memory, addr, self.get_used_dict_segment())?
        };

        loop {
            if current_article.name().as_bytes() == name {
                return Ok(Some(current_article));
            }

            current_article = match current_article.previous_article(self.get_used_dict_segment()) {
                Ok(Some(article)) => article,
                res => { return res; }
            };
        }
    }

    pub fn lookup_article_name_buf(&self, name_address: Address) -> Result<Option<ReadableArticle>, MemoryAccessError> {
        let s = ReadableSizedString::new(&self.raw_memory, name_address, self.raw_memory.address_range())?;

        self.lookup_article(s.as_bytes())
    }

    pub fn read_input_word(&mut self, input: &mut dyn Input) -> Result<Option<Address>, InputError> {
        let buffer_address = self.get_reserved_address(ReservedAddresses::WordBuffer);
        let content_address = buffer_address + 1;

        let word_length = input.read_word(self.raw_memory.address_slice_mut(content_address, 255))?.len();

        self.raw_memory.write_u8(buffer_address, word_length as u8);

        if word_length > 0 {
            Ok(Some(buffer_address))
        } else {
            Ok(None)
        }
    }

    pub fn copy_string(&mut self, src_address: Address, dst_address: Address, dst_segment: AddressRange) -> Result<(), MemoryAccessError> {
        let src_range = ReadableSizedString::new(&self.raw_memory, src_address, self.raw_memory.address_range())?.full_range();

        self.raw_memory.validate_access(
            dst_address..=(dst_address.wrapping_add((src_range.len() - 1) as u16)),
            dst_segment,
        )?;

        for src_byte_address in src_range {
            self.raw_memory.write_u8(
                src_byte_address - src_address + dst_address,
                self.raw_memory.read_u8(src_byte_address),
            )
        };

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn make_mem() -> MachineMemory {
        MachineMemory::new(Mem::default(), MemoryLayoutConfig::default())
    }

    #[test]
    fn test_data_stack() {
        let mut mm = make_mem();

        assert_eq!(mm.data_stack_depth(), 0);

        mm.data_push_u16(10500).unwrap();
        assert_eq!(mm.data_stack_depth(), 1);

        mm.data_push_u16(10501).unwrap();
        assert_eq!(mm.data_stack_depth(), 2);

        mm.data_push_u32(0xf000baaa).unwrap();
        assert_eq!(mm.data_stack_depth(), 4);

        assert_eq!(mm.data_pop_u32().unwrap(), 0xf000baaa);
        assert_eq!(mm.data_pop_u16().unwrap(), 10501);
        assert_eq!(mm.data_pop_u16().unwrap(), 10500);
        assert!(mm.data_pop_u16().is_err()); // Underflow
    }

    #[test]
    fn test_call_stack() {
        let mut mm = make_mem();

        mm.call_push_u16(0xdead).unwrap();
        mm.call_push_u16(0xc0de).unwrap();

        assert_eq!(mm.call_pop_u16().unwrap(), 0xc0de);
        assert_eq!(mm.call_pop_u16().unwrap(), 0xdead);
        assert!(mm.call_pop_u16().is_err()); // Underflow
    }

    #[test]
    fn test_call_stack_overflow() {
        let mut mm = make_mem();

        assert!(mm.call_pop_u16().is_err()); // Underflow, to ensure that stack pointer does not change

        for i in 0..MemoryLayoutConfig::default().max_call_stack_depth {
            mm.call_push_u16(i).unwrap();
        }

        assert!(mm.call_push_u16(0xdead).is_err());

        mm.call_pop_u16().unwrap();

        mm.call_push_u16(0x0000).unwrap();
    }

    #[test]
    fn test_reserved_variables() {
        let mm = make_mem();

        assert_eq!(
            unsafe { mm.raw_memory.read_u16(mm.get_reserved_address(ReservedAddresses::BaseVar)) },
            10
        );
    }
}
