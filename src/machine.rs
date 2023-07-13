use std::result::Result as StdResult;

use int_enum::IntEnum;

use crate::machine_memory::MachineMemory;
use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};
use crate::opcodes::OpCode;
use crate::readable_article::ReadableArticle;

#[derive(Debug)]
pub enum MachineError {
    MemoryAccessError(MemoryAccessError),
    IllegalInterpreterError {
        interpreter: u8,
    },
    IllegalOpCodeError {
        op_code: u8,
    },
    IllegalWord,
    IllegalMode {
        expected: MachineMode,
        actual: MachineMode,
    },
    Exited,
}

impl From<MemoryAccessError> for MachineError {
    fn from(err: MemoryAccessError) -> Self {
        MachineError::MemoryAccessError(err)
    }
}

type Result<T> = StdResult<T, MachineError>;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MachineMode {
    Interpreter,
    Compiler,
}

pub type WordFallbackHandler = fn(machine: &mut Machine, name_address: Address) -> Result<()>;

pub fn default_fallback_handler(_machine: &mut Machine, _name_address: Address) -> Result<()> {
    Err(MachineError::IllegalWord)
}

pub struct Machine {
    pub mode: MachineMode,

    pub word_fallback_handler: WordFallbackHandler,

    pub memory: MachineMemory
}

impl Machine {
    pub fn reset(&mut self) {
        self.memory.reset();
        self.mode = MachineMode::Interpreter;
    }

    pub fn expect_mode(&self, mode: MachineMode) -> Result<()> {
        if self.mode != mode {
            return Err(MachineError::IllegalMode {
                expected: mode,
                actual: self.mode.clone(),
            });
        }

        Ok(())
    }

    pub fn run_forever(&mut self, start_address: Address) -> Result<()> {
        let mut address = start_address;

        loop {
            address = OpCode::execute_at(self, address)?;
        }
    }

    pub fn run_until_exit(&mut self, start_address: Address) -> Result<()> {
        match self.run_forever(start_address) {
            Err(MachineError::Exited) => Ok(()),
            res => res
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            mode: MachineMode::Interpreter,
            word_fallback_handler: default_fallback_handler,
            memory: MachineMemory::default(),
        }
    }
}
