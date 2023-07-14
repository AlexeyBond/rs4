use int_enum::IntEnum;

use crate::literal::parse_literal;
use crate::machine::{Machine, MachineMode};
use crate::machine_error::MachineError;
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
        b":" => {
            machine.expect_mode(MachineMode::Interpreter)?;

            let name_buffer_address = machine.memory
                .read_input_word(machine.input.as_mut())?
                .ok_or(MachineError::UnexpectedInputEOF)?;

            let article_start_address = machine.memory.get_dict_ptr();
            let previous_article_address = machine.memory.last_article_ptr.unwrap_or(Address::MAX);

            machine.memory.dict_write_u16(previous_article_address)?;
            machine.memory.dict_write_sized_string(name_buffer_address)?;
            machine.memory.dict_write_opcode(OpCode::DefaultArticleStart)?;

            machine.memory.data_push_u16(article_start_address)?;

            machine.mode = MachineMode::Compiler;
        }
        b";" => {
            machine.expect_mode(MachineMode::Compiler)?;

            let article_start_address = machine.memory.data_pop_u16()?;

            machine.memory.dict_write_opcode(OpCode::Return)?;

            machine.memory.last_article_ptr = Some(article_start_address);
            machine.mode = MachineMode::Interpreter;
        }
        b"IMMEDIATE" => {
            machine.expect_mode(MachineMode::Interpreter)?;

            let body_address = machine.memory
                .articles().next()
                .ok_or(MachineError::NoArticle)?.body_address();

            if machine.memory.raw_memory.read_u8(body_address) != OpCode::DefaultArticleStart.int_value() {
                return Err(MachineError::UnexpectedArticleType);
            }

            machine.memory.raw_memory.write_u8(body_address, OpCode::Noop.int_value());
        }
        b"(" => {
            loop {
                match machine.input.read()? {
                    None => { return Err(MachineError::UnexpectedInputEOF); }
                    Some(b')') => { return Ok(()); }
                    Some(_) => { continue; }
                }
            }
        }
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
        b"HERE" => { process_constant(machine, machine.memory.get_reserved_address(ReservedAddresses::HereVar))?; }
        b"OVER" => { process_trivial_opcode(machine, OpCode::Over16)?; }
        b"2OVER" => { process_trivial_opcode(machine, OpCode::Over32)?; }
        b"SWAP" => { process_trivial_opcode(machine, OpCode::Swap16)?; }
        b"2SWAP" => { process_trivial_opcode(machine, OpCode::Swap32)?; }
        b"DUP" => { process_trivial_opcode(machine, OpCode::Dup16)?; }
        b"2DUP" => { process_trivial_opcode(machine, OpCode::Dup32)?; }
        b"DROP" => { process_trivial_opcode(machine, OpCode::Drop16)?; }
        b"2DROP" => {
            process_trivial_opcode(machine, OpCode::Drop16)?;
            process_trivial_opcode(machine, OpCode::Drop16)?;
        }
        b"+" => { process_trivial_opcode(machine, OpCode::Add16)?; }
        b"-" => { process_trivial_opcode(machine, OpCode::Sub16)?; }
        b"*" => { process_trivial_opcode(machine, OpCode::Mul16)?; }
        b"/" => { process_trivial_opcode(machine, OpCode::Div16)?; }
        b"@" => { process_trivial_opcode(machine, OpCode::Load16)?; }
        b"!" => { process_trivial_opcode(machine, OpCode::Store16)?; }
        b"C@" => { process_trivial_opcode(machine, OpCode::Load8)?; }
        b"C!" => { process_trivial_opcode(machine, OpCode::Store8)?; }
        b"2@" => { process_trivial_opcode(machine, OpCode::Load32)?; }
        b"2!" => { process_trivial_opcode(machine, OpCode::Store32)?; }
        b"EMIT" => { process_trivial_opcode(machine, OpCode::Emit)?; }
        _ => {
            return match (machine.word_fallback_handler)(machine, name_address) {
                Err(MachineError::IllegalWord(_)) => {
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
                        Err(MachineError::IllegalWord(Some(name_address)))
                    }
                }
                res => res
            };
        }
    };

    Ok(())
}
