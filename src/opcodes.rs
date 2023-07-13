use int_enum::IntEnum;

use crate::machine::{Machine, MachineError, MachineMode};
use crate::mem::Address;
use crate::readable_article::ReadableArticle;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, IntEnum)]
pub enum OpCode {
    Noop = 0,

    /// Op-code placed at beginning of a standard (non-immediate) article.
    ///
    /// Does nothing in interpreter mode (allowing seamless execution of following instructions).
    /// Writes a `Call` op-code with address of the next instruction to dictionary and returns in compiler mode.
    ///
    /// Can be replaced by `Noop` to make word executable both immediately and from compiled code.
    DefaultArticleStart = 1,

    /// Pop an address from call stack and go to that address.
    Return = 2,

    /// Must be followed by address of another instruction.
    ///
    /// Push an address immediately after this instruction (including address stored after it) to
    /// call stack, and go to that address.
    Call = 3,

    /// Must be followed by an 16-bit value.
    /// Pushes that value to data stack.
    Literal16 = 4,
}

impl OpCode {
    pub fn execute_at(machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        let op_code = machine.memory.raw_memory.read_u8(address);

        match OpCode::from_int(op_code) {
            Err(_) => Err(MachineError::IllegalOpCodeError { op_code }),
            Ok(op) => op.execute(machine, address)
        }
    }

    fn execute(self, machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        match self {
            OpCode::Noop => {
                Ok(address + 1)
            }

            OpCode::DefaultArticleStart => {
                match machine.mode {
                    MachineMode::Interpreter => {
                        Ok(address + 1) // Noop
                    }
                    MachineMode::Compiler => {
                        machine.memory.dict_write_opcode(OpCode::Call)?;
                        machine.memory.dict_write_u16(address + 1)?;
                        machine.memory.call_pop_u16()
                    }
                }
            }

            OpCode::Return => {
                machine.memory.call_pop_u16()
            }

            OpCode::Call => {
                machine.memory.raw_memory.validate_access(
                    address + 1..=address + 2,
                    machine.memory.get_used_dict_segment(),
                )?;

                let target_address = unsafe { machine.memory.raw_memory.read_u16(address) };

                machine.memory.call_push_u16(address + 3)?;

                Ok(target_address)
            }

            OpCode::Literal16 => {
                machine.memory.raw_memory.validate_access(
                    address + 1..=address + 2,
                    machine.memory.get_used_dict_segment(),
                )?;

                let literal = unsafe { machine.memory.raw_memory.read_u16(address + 1) };

                machine.memory.data_push_u16(literal)?;

                Ok(address + 3)
            }
        }.map_err(|err| err.into())
    }
}
