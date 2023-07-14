use int_enum::IntEnum;

use crate::machine::{Machine, MachineMode};
use crate::machine_error::MachineError;
use crate::mem::Address;
use crate::sized_string::ReadableSizedString;

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

    /// Must be followed by a sized string.
    /// Pushes address of that string to data stack.
    LiteralString = 5,

    /// Must be followed by an 16-bit address of another instruction.
    ///
    /// Unconditionally goes to that address.
    GoTo = 6,

    /// Must be followed by an 16-bit address of another instruction.
    ///
    /// Takes one cell from data stack and goes to that address iff value of that cell is zero.
    GoToIfZ = 7,

    Dup32 = 123,
    Over16 = 124,
    Over32 = 125,
    Swap16 = 126,
    Swap32 = 127,
    Dup16 = 128,
    Add16 = 129,
    Sub16 = 130,
    Mul16 = 131,
    Div16 = 132,
    Load16 = 133,
    Store16 = 134,
    Load8 = 135,
    Store8 = 136,
    Load32 = 137,
    Store32 = 138,
    Drop16 = 139,
    Invert16 = 140,
    And16 = 141,
    Or16 = 142,
    Xor16 = 143,

    Emit = 200,
}

impl OpCode {
    pub fn execute_at(machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        let op_code = machine.memory.raw_memory.read_u8(address);

        match OpCode::from_int(op_code) {
            Err(_) => Err(MachineError::IllegalOpCodeError { address, op_code }),
            Ok(op) => op.execute(machine, address)
        }
    }

    pub fn execute(self, machine: &mut Machine, address: Address) -> Result<Address, MachineError> {
        Ok(match self {
            OpCode::Noop => {
                address + 1
            }

            OpCode::DefaultArticleStart => {
                match machine.mode {
                    MachineMode::Interpreter => {
                        address + 1 // Noop
                    }
                    MachineMode::Compiler => {
                        machine.memory.dict_write_opcode(OpCode::Call)?;
                        machine.memory.dict_write_u16(address + 1)?;
                        machine.memory.call_pop_u16()?
                    }
                }
            }

            OpCode::Return => {
                if machine.memory.call_stack_depth() == 0 {
                    return Err(MachineError::Exited);
                }

                machine.memory.call_pop_u16()?
            }

            OpCode::Call => {
                machine.memory.raw_memory.validate_access(
                    address + 1..=address + 2,
                    machine.memory.get_used_dict_segment(),
                )?;

                let target_address = unsafe { machine.memory.raw_memory.read_u16(address) };

                machine.memory.call_push_u16(address + 3)?;

                target_address
            }

            OpCode::Literal16 => {
                machine.memory.raw_memory.validate_access(
                    address + 1..=address + 2,
                    machine.memory.get_used_dict_segment(),
                )?;

                let literal = unsafe { machine.memory.raw_memory.read_u16(address + 1) };

                machine.memory.data_push_u16(literal)?;

                address + 3
            }

            OpCode::GoTo => {
                machine.memory.raw_memory.validate_access(
                    address + 1..=address + 2,
                    machine.memory.get_used_dict_segment(),
                )?;

                unsafe { machine.memory.raw_memory.read_u16(address) }
            }

            OpCode::GoToIfZ => {
                let value = machine.memory.data_pop_u16()?;

                if value == 0 {
                    machine.memory.raw_memory.validate_access(
                        address + 1..=address + 2,
                        machine.memory.get_used_dict_segment(),
                    )?;

                    unsafe { machine.memory.raw_memory.read_u16(address) }
                } else {
                    address + 3
                }
            }

            OpCode::LiteralString => {
                let string_range = ReadableSizedString::new(
                    &machine.memory.raw_memory,
                    address + 1,
                    machine.memory.get_used_dict_segment(),
                )?.full_range();

                machine.memory.data_push_u16(*string_range.start())?;

                string_range.end().wrapping_add(1)
            }

            OpCode::Over16 => {
                todo!()
            }

            OpCode::Over32 => {
                todo!()
            }

            OpCode::Swap16 => {
                let a = machine.memory.data_pop_u16()?;
                let b = machine.memory.data_pop_u16()?;

                machine.memory.data_push_u16(a)?;
                machine.memory.data_push_u16(b)?;

                address + 1
            }

            OpCode::Swap32 => {
                let a = machine.memory.data_pop_u32()?;
                let b = machine.memory.data_pop_u32()?;

                machine.memory.data_push_u32(a)?;
                machine.memory.data_push_u32(b)?;

                address + 1
            }

            OpCode::Dup16 => {
                let val = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(val)?;
                machine.memory.data_push_u16(val)?;

                address + 1
            }

            OpCode::Dup32 => {
                let val = machine.memory.data_pop_u32()?;
                machine.memory.data_push_u32(val)?;
                machine.memory.data_push_u32(val)?;

                address + 1
            }

            OpCode::Drop16 => {
                machine.memory.data_pop_u16()?;

                address + 1
            }

            OpCode::Add16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a.wrapping_add(b))?;

                address + 1
            }

            OpCode::Sub16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a.wrapping_sub(b))?;

                address + 1
            }

            OpCode::Mul16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a.wrapping_mul(b))?;

                address + 1
            }

            OpCode::Div16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a.wrapping_div(b))?;

                address + 1
            }

            OpCode::Load8 => {
                let address = machine.memory.data_pop_u16()? as Address;

                machine.memory.raw_memory.validate_access(
                    address..=address,
                    machine.memory.raw_memory.address_range(),
                )?;

                let value = machine.memory.raw_memory.read_u8(address);

                machine.memory.data_push_u16(value as u16)?;

                address + 1
            }

            OpCode::Store8 => {
                let address = machine.memory.data_pop_u16()? as Address;
                let value = machine.memory.data_pop_u16()?;

                machine.memory.raw_memory.validate_access(
                    address..=address,
                    machine.memory.raw_memory.address_range(),
                )?;

                machine.memory.raw_memory.write_u8(address, value as u8);

                address + 1
            }

            OpCode::Load16 => {
                let address = machine.memory.data_pop_u16()? as Address;

                machine.memory.raw_memory.validate_access(
                    address..=address.wrapping_add(1),
                    machine.memory.raw_memory.address_range(),
                )?;

                let value = unsafe { machine.memory.raw_memory.read_u16(address) };

                machine.memory.data_push_u16(value)?;

                address + 1
            }

            OpCode::Store16 => {
                let address = machine.memory.data_pop_u16()? as Address;
                let value = machine.memory.data_pop_u16()?;

                machine.memory.raw_memory.validate_access(
                    address..=address.wrapping_add(1),
                    machine.memory.raw_memory.address_range(),
                )?;

                unsafe { machine.memory.raw_memory.write_u16(address, value) };

                address + 1
            }

            OpCode::Load32 => {
                let address = machine.memory.data_pop_u16()? as Address;

                machine.memory.raw_memory.validate_access(
                    address..=address.wrapping_add(3),
                    machine.memory.raw_memory.address_range(),
                )?;

                let value = unsafe { machine.memory.raw_memory.read_u32(address) };

                machine.memory.data_push_u32(value)?;

                address + 1
            }

            OpCode::Store32 => {
                let address = machine.memory.data_pop_u16()? as Address;
                let value = machine.memory.data_pop_u32()?;

                machine.memory.raw_memory.validate_access(
                    address..=address.wrapping_add(3),
                    machine.memory.raw_memory.address_range(),
                )?;

                unsafe { machine.memory.raw_memory.write_u32(address, value) };

                address + 1
            }

            OpCode::Invert16 => {
                let val = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(!val)?;

                address + 1
            }

            OpCode::And16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a & b)?;

                address + 1
            }

            OpCode::Or16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a | b)?;

                address + 1
            }

            OpCode::Xor16 => {
                let b = machine.memory.data_pop_u16()?;
                let a = machine.memory.data_pop_u16()?;
                machine.memory.data_push_u16(a ^ b)?;

                address + 1
            }

            OpCode::Emit => {
                let char_code = machine.memory.data_pop_u16()?;

                machine.output.putc(char_code)?;

                address + 1
            }
        })
    }
}
