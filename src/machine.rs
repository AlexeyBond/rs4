use std::result::Result as StdResult;

use crate::builtin_words::process_builtin_word;
use crate::input::{EmptyInput, Input, InputError};
use crate::machine_memory::MachineMemory;
use crate::mem::{Address, MemoryAccessError};
use crate::opcodes::OpCode;

#[derive(Debug)]
pub enum MachineError {
    MemoryAccessError(MemoryAccessError),
    InputError(InputError),
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

impl From<InputError> for MachineError {
    fn from(err: InputError) -> Self {
        MachineError::InputError(err)
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
    pub input: Box<dyn Input>,

    pub mode: MachineMode,

    pub word_fallback_handler: WordFallbackHandler,

    pub memory: MachineMemory,
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

    pub fn execute_word(&mut self, name_address: Address) -> Result<()> {
        if let Some(article) = self.memory.lookup_article_name_buf(name_address)? {
            self.run_until_exit(article.body_address())
        } else {
            process_builtin_word(self, name_address)
        }
    }

    pub fn interpret_input(&mut self) -> Result<()> {
        loop {
            if let Some(name_address) = self.memory.read_input_word(self.input.as_mut())? {
                self.execute_word(name_address)?;
            } else {
                return Ok(())
            }
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            input: Box::new(EmptyInput {}),
            mode: MachineMode::Interpreter,
            word_fallback_handler: default_fallback_handler,
            memory: MachineMemory::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::machine_testing::*;

    use super::*;

    fn test_16_bit_results(input: &'static str, results: &[u16]) {
        Machine::run_with_test_input(input)
            .unwrap()
            .assert_data_stack_state(&results.iter().map(|r| StackElement::Cell(*r)).collect::<Vec<_>>())
    }

    #[test]
    fn test_push_literal() {
        test_16_bit_results("1 2", &[1, 2]);
    }

    #[test]
    fn test_arithmetic() {
        test_16_bit_results("1 2 +", &[3]);
        test_16_bit_results("1 -3 -", &[4]);
        test_16_bit_results("42 2 *", &[84]);
        test_16_bit_results("10 2 /", &[5]);
    }

    #[test]
    fn test_store_load_16() {
        test_16_bit_results("42 101 !", &[]);
        test_16_bit_results("42 101 ! 101 @", &[42]);
    }

    #[test]
    fn test_store_load_8() {
        test_16_bit_results("$FFFF 101 C! $FEFE 102 C!", &[]);
        test_16_bit_results("$FFFF 101 C! $FEFE 102 C! 101 C@ 102 C@", &[0xff, 0xfe]);
    }

    #[test]
    fn test_radix_change() {
        test_16_bit_results("100 36 BASE ! zZz", &[100, 46655]);
    }
}
