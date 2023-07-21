use int_enum::IntEnum;

use crate::literal::parse_literal;
use crate::machine::Machine;
use crate::machine_error::MachineError;
use crate::machine_memory::ReservedAddresses;
use crate::machine_state::MachineState;
use crate::mem::{Address, MemoryAccessError};
use crate::opcodes::OpCode;
use crate::readable_article::ReadableArticle;
use crate::sized_string::{ReadableSizedString, SizedStringWriter};
use crate::stack_effect::stack_effect;

fn compile_u16_literal(machine: &mut Machine, value: u16) -> Result<(), MemoryAccessError> {
    machine.memory.dict_write_opcode(OpCode::Literal16)?;
    machine.memory.dict_write_u16(value)
}

fn process_literal(machine: &mut Machine, value: u16) -> Result<(), MemoryAccessError> {
    match machine.memory.get_state() {
        MachineState::Interpreter => machine.memory.data_push_u16(value),
        MachineState::Compiler => compile_u16_literal(machine, value)
    }
}

pub fn process_trivial_opcode(machine: &mut Machine, opcode: OpCode) -> Result<(), MachineError> {
    match machine.memory.get_state() {
        MachineState::Interpreter => {
            let next_address = opcode.execute(machine, 0)?;

            debug_assert_eq!(
                next_address, 1,
                "Unexpected address returned from trivial opcode {:?}", opcode,
            );
        }

        MachineState::Compiler => {
            machine.memory.dict_write_opcode(opcode)?;
        }
    };

    Ok(())
}

pub fn process_compile_only_opcode(machine: &mut Machine, opcode: OpCode) -> Result<(), MachineError> {
    machine.expect_state(MachineState::Compiler)?;

    Ok(machine.memory.dict_write_opcode(opcode)?)
}

pub fn compile_string_literal(machine: &mut Machine) -> Result<(), MachineError> {
    machine.memory.dict_write_opcode(OpCode::LiteralString)?;

    let start_address = machine.memory.get_dict_ptr();
    let safe_range = machine.memory.get_free_data_segment();
    let mut writer = SizedStringWriter::new(&mut machine.memory.raw_memory, start_address, u8::MAX, safe_range)?;

    loop {
        let ch = machine.input.read()?.ok_or(MachineError::UnexpectedInputEOF)?;

        if ch == b'"' {
            break;
        }

        writer.append_u8(ch)?;
    }

    let end_address = writer.finish().full_range().end().wrapping_add(1);
    machine.memory.set_dict_ptr(end_address);

    Ok(())
}

pub fn process_constant(machine: &mut Machine, value: u16) -> Result<(), MachineError> {
    match machine.memory.get_state() {
        MachineState::Interpreter => machine.memory.data_push_u16(value)?,
        MachineState::Compiler => {
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
            machine.expect_state(MachineState::Interpreter)?;

            if let Some(_) = machine.memory.get_current_word() {
                return Err(MachineError::IllegalCompilerState);
            }

            let name_buffer_address = machine.memory
                .read_input_word(machine.input.as_mut())?
                .ok_or(MachineError::UnexpectedInputEOF)?;

            let article_start_address = machine.memory.get_dict_ptr();
            let previous_article_address = machine.memory.last_article_ptr.unwrap_or(Address::MAX);

            machine.memory.dict_write_u16(previous_article_address)?;
            machine.memory.dict_write_sized_string(name_buffer_address)?;
            machine.memory.dict_write_opcode(OpCode::DefaultArticleStart)?;

            machine.memory.set_current_word(Some(article_start_address));

            machine.memory.set_state(MachineState::Compiler);
        }
        b";" => {
            machine.expect_state(MachineState::Compiler)?;
            let article_start_address = machine.memory.get_current_word().ok_or(MachineError::IllegalCompilerState)?;

            machine.memory.dict_write_opcode(OpCode::Return)?;

            machine.memory.last_article_ptr = Some(article_start_address);
            machine.memory.set_current_word(None);
            machine.memory.set_state(MachineState::Interpreter);
        }
        b"RECURSE" => {
            machine.expect_state(MachineState::Compiler)?;
            let article_header_address = machine.memory.get_current_word().ok_or(MachineError::IllegalCompilerState)?;
            let article_body_address = ReadableArticle::new(
                &machine.memory.raw_memory,
                article_header_address,
                machine.memory.get_used_dict_segment(),
            )?.body_address();

            machine.memory.dict_write_opcode(OpCode::Call)?;
            machine.memory.dict_write_u16(article_body_address)?;
        }
        b"IMMEDIATE" => {
            machine.expect_state(MachineState::Interpreter)?;

            let body_address = machine.memory
                .articles().next()
                .ok_or(MachineError::NoArticle)?.body_address();

            if machine.memory.raw_memory.read_u8(body_address) != OpCode::DefaultArticleStart.int_value() {
                return Err(MachineError::UnexpectedArticleType);
            }

            machine.memory.raw_memory.write_u8(body_address, OpCode::Noop.int_value());
        }
        b"IF" => {
            machine.expect_state(MachineState::Compiler)?;

            machine.memory.dict_write_opcode(OpCode::GoToIfZ)?;
            let forward_ref = machine.memory.create_forward_reference()?;
            machine.memory.data_push_u16(forward_ref)?;
        }
        b"ELSE" => {
            machine.expect_state(MachineState::Compiler)?;

            let mut fx = stack_effect!(machine; old_ref:Address => new_ref: Address)?;
            let old_ref = fx.old_ref();

            fx.machine.memory.dict_write_opcode(OpCode::GoTo)?;
            let new_ref = fx.machine.memory.create_forward_reference()?;
            fx.new_ref(new_ref);
            fx.machine.memory.resolve_forward_reference(old_ref)?;

            fx.commit();
        }
        b"THEN" => {
            machine.expect_state(MachineState::Compiler)?;

            let reference = machine.memory.data_pop_u16()?;
            machine.memory.resolve_forward_reference(reference)?;
        }
        b"BEGIN" => {
            machine.expect_state(MachineState::Compiler)?;

            machine.memory.data_push_u16(machine.memory.get_dict_ptr())?;
        }
        b"WHILE" => {
            let mut fx = stack_effect!(machine; old_dest: Address => orig: Address, new_dest: Address)?;
            let dest = fx.old_dest();
            fx.new_dest(dest);

            fx.machine.memory.dict_write_opcode(OpCode::GoToIfZ)?;
            let orig = fx.machine.memory.create_forward_reference()?;
            fx.orig(orig);
            fx.commit();
        }
        b"REPEAT" => {
            let fx = stack_effect!(machine; orig: Address, dest: Address => )?;
            let (dest, orig) = (fx.dest(), fx.orig());

            fx.machine.memory.dict_write_opcode(OpCode::GoTo)?;
            fx.machine.memory.dict_write_u16(dest)?;
            fx.machine.memory.resolve_forward_reference(orig)?;

            fx.commit();
        }
        b"EXIT" => {
            machine.expect_state(MachineState::Compiler)?;

            machine.memory.dict_write_opcode(OpCode::Return)?;
        }
        b"POSTPONE" => {
            let name_address = machine.read_input_word()?.ok_or(MachineError::UnexpectedInputEOF)?;

            if let Some(article) = machine.memory.lookup_article_name_buf(name_address)? {
                let body_address = article.body_address();

                machine.memory.dict_write_opcode(OpCode::Call)?;
                machine.memory.dict_write_u16(body_address)?;
            } else {
                machine.memory.dict_write_opcode(OpCode::ExecBuiltin)?;
                machine.memory.dict_write_sized_string(name_address)?;
            }
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
            machine.expect_state(MachineState::Compiler)?;
            machine.memory.set_state(MachineState::Interpreter);
        }
        b"]" => {
            machine.expect_state(MachineState::Interpreter)?;
            machine.memory.set_state(MachineState::Compiler);
        }
        b"TRUE" => { process_constant(machine, TRUE)?; }
        b"FALSE" => { process_constant(machine, FALSE)?; }
        b"BASE" => { process_constant(machine, machine.memory.get_reserved_address(ReservedAddresses::BaseVar))?; }
        b"HERE" => { process_constant(machine, machine.memory.get_reserved_address(ReservedAddresses::HereVar))?; }
        b"STATE" => { process_constant(machine, machine.memory.get_reserved_address(ReservedAddresses::StateVar))?; }
        b"PAD" => { process_literal(machine, machine.memory.get_reserved_address(ReservedAddresses::PadBuffer))?; }
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
        b"ROT" => { process_trivial_opcode(machine, OpCode::Rot16)?; }
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
        b"<" => { process_trivial_opcode(machine, OpCode::Lt16)?; }
        b">" => { process_trivial_opcode(machine, OpCode::Gt16)?; }
        b"=" => { process_trivial_opcode(machine, OpCode::Eq16)?; }
        b"INVERT" => { process_trivial_opcode(machine, OpCode::Invert16)?; }
        b"AND" => { process_trivial_opcode(machine, OpCode::And16)?; }
        b"OR" => { process_trivial_opcode(machine, OpCode::Or16)?; }
        b"XOR" => { process_trivial_opcode(machine, OpCode::Xor16)?; }
        b"S>D" => { process_trivial_opcode(machine, OpCode::I16ToI32)?; }
        b"R@" => { process_compile_only_opcode(machine, OpCode::CallRead16)?; }
        b"2R@" => { process_compile_only_opcode(machine, OpCode::CallRead32)?; }
        b">R" => { process_compile_only_opcode(machine, OpCode::CallPush16)?; }
        b"R>" => { process_compile_only_opcode(machine, OpCode::CallPop16)?; }
        b"2>R" => { process_compile_only_opcode(machine, OpCode::CallPush32)?; }
        b"2R>" => { process_compile_only_opcode(machine, OpCode::CallPop32)?; }
        b"ABS" => { process_trivial_opcode(machine, OpCode::Abs16)?; }
        b"S\"" => {
            machine.expect_state(MachineState::Compiler)?;

            compile_string_literal(machine)?;
        }
        b"LITERAL" => {
            machine.expect_state(MachineState::Compiler)?;

            let value = machine.memory.data_pop_u16()?;
            compile_u16_literal(machine, value)?;
        }
        b"EMIT" => { process_trivial_opcode(machine, OpCode::Emit)?; }
        b"TYPE" => { process_trivial_opcode(machine, OpCode::EmitString)?; }
        b"<#" => { process_trivial_opcode(machine, OpCode::PnoInit)?; }
        b"HOLD" => { process_trivial_opcode(machine, OpCode::PnoPut)?; }
        b"#>" => { process_trivial_opcode(machine, OpCode::PnoFinish)?; }
        b"#" => { process_trivial_opcode(machine, OpCode::PnoPutDigit)?; }
        b".\"" => {
            match machine.memory.get_state() {
                MachineState::Compiler => {
                    compile_string_literal(machine)?;
                    machine.memory.dict_write_opcode(OpCode::EmitString)?;
                }
                MachineState::Interpreter => {
                    loop {
                        let c = machine.input.read()?.ok_or(MachineError::UnexpectedInputEOF)?;

                        if c == b'"' {
                            break
                        }

                        machine.output.putc(c as u16)?;
                    }
                }
            }
        }
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
                        Ok(process_literal(machine, parsed_literal)?)
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
