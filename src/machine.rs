use std::result::Result as StdResult;

use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};
use crate::memory_segment::{CallStackSegment, DataStackSegment, FreeDataSegment, MemorySegment, UsedDictionarySegment};
use crate::readable_article::ReadableArticle;

#[derive(Debug)]
pub enum MachineError {
    MemoryAccessError(&'static str, MemoryAccessError),
    IllegalInterpreterError {
        interpreter: u8,
    },
    IllegalOpCodeError {
        op_code: u8,
    },
    IllegalWord,
}

type Result<T> = StdResult<T, MachineError>;

pub enum MachineMode {
    Interpreter,
    Compiler,
}

pub type WordFallbackHandler = fn(machine: &mut Machine, name_address: Address) -> Result<()>;

pub fn default_fallback_handler(_machine: &mut Machine, _name_address: Address) -> Result<()> {
    Err(MachineError::IllegalWord)
}

pub struct Machine {
    /// Address of next free dictionary byte
    pub dict_ptr: Address,

    /// Address immediately before the last element on call stack
    pub call_stack_ptr: Address,

    /// Address of the last byte (one with lowest address) available for call stack
    pub stacks_border: Address,

    /// Address of last element on data stack
    pub data_stack_ptr: Address,

    /// Address of the last article in the dictionary, `None` if no articles were written
    pub last_article_ptr: Option<Address>,

    /// Address of the  next instruction to execute
    pub instruction_ptr: Address,

    pub mode: MachineMode,

    pub word_fallback_handler: WordFallbackHandler,

    /// Memory of the virtual machine
    pub memory: Mem,
}

impl Machine {
    pub fn reset(&mut self, call_stack_depth: u16) {
        let address_range = self.memory.address_range();

        self.dict_ptr = *address_range.start();
        self.call_stack_ptr = *address_range.end();
        self.stacks_border = *address_range.end() - 2 * call_stack_depth;
        self.data_stack_ptr = self.stacks_border;
        self.last_article_ptr = None;
        self.instruction_ptr = 0;
        self.mode = MachineMode::Interpreter;
    }

    pub fn validate_access<TSeg: MemorySegment>(&self, segment: &TSeg, access_range: AddressRange) -> Result<()> {
        self.memory
            .validate_access(access_range, segment.get_range(self))
            .map_err(|err| MachineError::MemoryAccessError(TSeg::NAME, err))
    }

    pub fn dict_write_u8(
        &mut self,
        value: u8,
    ) -> Result<()> {
        self.validate_access(
            &FreeDataSegment {},
            self.dict_ptr..=self.dict_ptr,
        )?;

        self.memory.write_u8(self.dict_ptr, value);

        self.dict_ptr += 1;

        Ok(())
    }

    pub fn dict_write_u16(
        &mut self,
        value: u16,
    ) -> Result<()> {
        self.validate_access(
            &FreeDataSegment {},
            self.dict_ptr..=(self.dict_ptr + 1),
        )?;

        unsafe { self.memory.write_u16(self.dict_ptr, value) }

        self.dict_ptr += 2;

        Ok(())
    }

    pub fn data_push_u16(
        &mut self,
        value: u16,
    ) -> Result<()> {
        let next_stack_ptr = self.data_stack_ptr - 2;

        self.validate_access(
            &DataStackSegment {},
            next_stack_ptr..=next_stack_ptr + 1,
        )?;

        unsafe { self.memory.write_u16(next_stack_ptr, value) }

        self.data_stack_ptr = next_stack_ptr;

        Ok(())
    }

    pub fn data_pop_u16(&mut self) -> Result<u16> {
        self.validate_access(
            &DataStackSegment {},
            self.data_stack_ptr..=self.data_stack_ptr + 1,
        )?;

        let result = unsafe { self.memory.read_u16(self.data_stack_ptr) };

        self.data_stack_ptr += 2;

        Ok(result)
    }

    pub fn call(&mut self, address: Address) -> Result<()> {
        let call_address = self.call_stack_ptr - 1;

        self.validate_access(
            &CallStackSegment {},
            call_address..=call_address + 1,
        )?;

        unsafe { self.memory.write_u16(call_address, self.instruction_ptr) };

        self.call_stack_ptr = call_address - 1;
        self.instruction_ptr = address;

        Ok(())
    }

    pub fn ret(&mut self) -> Result<bool> {
        if self.call_stack_ptr == *self.memory.address_range().end() {
            return Ok(true);
        }

        self.validate_access(
            &CallStackSegment {},
            self.call_stack_ptr + 1..=self.call_stack_ptr + 2,
        )?;

        self.instruction_ptr = unsafe {
            self.memory.read_u16(self.call_stack_ptr + 1)
        };

        self.call_stack_ptr += 2;

        Ok(false)
    }

    pub fn lookup_article(&self, name: &[u8]) -> Result<Option<ReadableArticle>> {
        let safe_range = (UsedDictionarySegment {}).get_range(self);

        let mut current_article = match self.last_article_ptr {
            None => { return Ok(None); }
            Some(addr) => ReadableArticle::new(&self.memory, addr, safe_range.clone())
                .map_err(|err| MachineError::MemoryAccessError(UsedDictionarySegment::NAME, err))?
        };

        loop {
            if current_article.name().as_bytes() == name {
                return Ok(Some(current_article));
            }

            current_article = match current_article.previous_article(safe_range.clone()) {
                Ok(Some(article)) => article,
                Ok(None) => { return Ok(None); }
                Err(err) => { return Err(MachineError::MemoryAccessError(UsedDictionarySegment::NAME, err)) }
            };
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        let mut machine = Machine {
            dict_ptr: 0,
            call_stack_ptr: 0,
            stacks_border: 0,
            data_stack_ptr: 0,
            last_article_ptr: None,
            instruction_ptr: 0,
            mode: MachineMode::Interpreter,
            word_fallback_handler: default_fallback_handler,
            memory: Default::default(),
        };

        machine.reset(64);

        machine
    }
}
