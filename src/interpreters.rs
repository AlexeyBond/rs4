use int_enum::IntEnum;

use crate::machine::{Machine, MachineError, MachineMode};
use crate::mem::Address;
use crate::opcodes::OpCode;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, IntEnum)]
pub enum Interpreter {
    Default = 0,
    Immediate = 1,
}

impl Interpreter {
    pub fn interpret_article(machine: &mut Machine, address: Address) -> Result<(), MachineError> {
        let interpreter_code = machine.memory.read_u8(address);

        match Interpreter::from_int(interpreter_code) {
            Ok(interpreter) => interpreter.interpret(machine, address),
            Err(_) => Err(MachineError::IllegalInterpreterError { interpreter: interpreter_code })
        }
    }

    fn interpret(self, machine: &mut Machine, address: Address) -> Result<(), MachineError> {
        match self {
            Interpreter::Default => match machine.mode {
                MachineMode::Compiler => {
                    machine.dict_write_u8(OpCode::CallArticle.int_value())?;
                    machine.dict_write_u16(address)
                },
                MachineMode::Interpreter => {
                    machine.call(address + 1)
                }
            },
            Interpreter::Immediate => machine.call(address + 1),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_integer() {
        assert_eq!(
            Interpreter::from_int(0),
            Ok(Interpreter::Default),
        );

        assert!(
            Interpreter::from_int(255).is_err(),
        );
    }
}
