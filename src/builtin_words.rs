use crate::literal::parse_literal;
use crate::machine::{Machine, MachineError, MachineMode};
use crate::machine_memory::ReservedAddresses;
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

pub fn process_trivial_opcode(machine: &mut Machine, opcode: OpCode) -> Result<(), MachineError> {
    match machine.mode {
        MachineMode::Interpreter => {
            opcode.execute(machine, 0)?;
        }

        MachineMode::Compiler => {
            machine.memory.dict_write_opcode(opcode)?;
        }
    };

    Ok(())
}

pub fn process_constant(machine: &mut Machine, value: u16) -> Result<(), MachineError> {
    match machine.mode {
        MachineMode::Interpreter => machine.memory.data_push_u16(value)?,
        MachineMode::Compiler => {
            machine.memory.dict_write_opcode(OpCode::Literal16)?;
            machine.memory.dict_write_u16(value)?
        }
    }

    Ok(())
}

const TRUE: u16 = 0xFFFF;
const FALSE: u16 = 0;

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
        b"TRUE" => { process_constant(machine, TRUE)?; }
        b"FALSE" => { process_constant(machine, FALSE)?; }
        b"BASE" => { process_constant(machine, machine.memory.get_reserved_address(ReservedAddresses::BaseVar))?; }
        b"DROP" => { process_trivial_opcode(machine, OpCode::Dup16)?; }
        b"+" => { process_trivial_opcode(machine, OpCode::Add16)?; }
        b"-" => { process_trivial_opcode(machine, OpCode::Sub16)?; }
        b"*" => { process_trivial_opcode(machine, OpCode::Mul16)?; }
        b"/" => { process_trivial_opcode(machine, OpCode::Div16)?; }
        b"@" => { process_trivial_opcode(machine, OpCode::Load16)?; }
        b"!" => { process_trivial_opcode(machine, OpCode::Store16)?; }
        b"C@" => { process_trivial_opcode(machine, OpCode::Load8)?; }
        b"C!" => { process_trivial_opcode(machine, OpCode::Store8)?; }
        _ => {
            return match (machine.word_fallback_handler)(machine, name_address) {
                Err(MachineError::IllegalWord) => {
                    let base_address = machine.memory.get_reserved_address(ReservedAddresses::BaseVar);
                    let base = unsafe { machine.memory.raw_memory.read_u16(base_address) };

                    if let Some(parsed_literal) = parse_literal(
                        ReadableSizedString::new(
                            &machine.memory.raw_memory,
                            name_address,
                            machine.memory.raw_memory.address_range(),
                        )?
                            .as_bytes(),
                        base as u32,
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
