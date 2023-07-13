use crate::literal::parse_literal;
use crate::machine::{Machine, MachineError, MachineMode};
use crate::mem::Address;
use crate::opcodes::OpCode;
use crate::sized_string::ReadableSizedString;

fn process_literal(machine: &mut Machine, value: u16) -> Result<(), MachineError> {
    match machine.mode {
        MachineMode::Interpreter => machine.memory.data_push_u16(value),
        MachineMode::Compiler => {
            machine.memory.dict_write_opcode(OpCode::Literal16)?;
            machine.memory.dict_write_u16(value)
        }
    }.map_err(|err| err.into())
}

pub fn process_builtin_word(machine: &mut Machine, name_address: Address) -> Result<(), MachineError> {
    match ReadableSizedString::new(&machine.memory.raw_memory, name_address, machine.memory.raw_memory.address_range())?
        .as_bytes() {
        b"[" => {
            machine.expect_mode(MachineMode::Compiler)?;
            machine.mode = MachineMode::Interpreter;
        }
        b"]" => {
            machine.expect_mode(MachineMode::Interpreter)?;
            machine.mode = MachineMode::Compiler;
        }
        b"DROP" => {
            todo!("execute or write opcode")
        }
        _ => {
            return match (machine.word_fallback_handler)(machine, name_address) {
                Err(MachineError::IllegalWord) => {
                    if let Some(parsed_literal) = parse_literal(
                        ReadableSizedString::new(
                            &machine.memory.raw_memory,
                            name_address,
                            machine.memory.raw_memory.address_range(),
                        )?
                            .as_bytes(),
                        10, // TODO
                    ) {
                        process_literal(machine, parsed_literal)
                    } else {
                        Err(MachineError::IllegalWord)
                    }
                }
                res => res
            };
        }
    };

    Ok(())
}
