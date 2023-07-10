use int_enum::IntEnum;

use crate::interpreters::Interpreter;
use crate::machine::{Machine, MachineError};
use crate::mem::Address;
use crate::memory_segment::UsedDictionarySegment;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, IntEnum)]
pub enum OpCode {
    CallArticle = 0,
    Literal16 = 1,
}

impl OpCode {
    fn execute_at(machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        let op_code = machine.memory.read_u8(address);

        match OpCode::from_int(op_code) {
            Err(_) => Err(MachineError::IllegalOpCodeError { op_code }),
            Ok(op) => op.execute(machine, address + 1)
        }
    }

    fn execute(self, machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        match self {
            OpCode::CallArticle => {
                machine.validate_access(
                    &UsedDictionarySegment {},
                    address..=address + 1,
                )?;

                let article_address = unsafe { machine.memory.read_u16(address) };

                Interpreter::interpret_article(machine, article_address)?;

                Ok(address + 2)
            }

            OpCode::Literal16 => {
                machine.validate_access(
                    &UsedDictionarySegment {},
                    address..=address + 1,
                )?;

                let literal = unsafe { machine.memory.read_u16(address) };

                machine.data_push_u16(literal)?;

                Ok(address + 2)
            }
        }
    }
}
