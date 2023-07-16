use std::io;
use std::str::from_utf8;
use int_enum::IntEnum;
use crate::builtin_words::process_builtin_word;

use crate::machine::{Machine, MachineMode};
use crate::machine_error::MachineError;
use crate::mem::{Address};
use crate::sized_string::ReadableSizedString;
use crate::stack_effect::stack_effect;

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

    /// Must be followed by a sized string.
    /// Executes a built-in word with name contained in that string.
    ExecBuiltin = 8,

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
    Eq16 = 144,
    Lt16 = 145,
    Gt16 = 146,

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

                        if machine.memory.call_stack_depth() == 0 {
                            return Err(MachineError::Exited);
                        }

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

                let target_address = unsafe { machine.memory.raw_memory.read_u16(address + 1) };

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

                unsafe { machine.memory.raw_memory.read_u16(address + 1) }
            }

            OpCode::GoToIfZ => {
                let value = machine.memory.data_pop_u16()?;

                if value == 0 {
                    machine.memory.raw_memory.validate_access(
                        address + 1..=address + 2,
                        machine.memory.get_used_dict_segment(),
                    )?;

                    unsafe { machine.memory.raw_memory.read_u16(address + 1) }
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

            OpCode::ExecBuiltin => {
                let string_range = ReadableSizedString::new(
                    &machine.memory.raw_memory,
                    address + 1,
                    machine.memory.get_used_dict_segment(),
                )?.full_range();

                process_builtin_word(machine, *string_range.start())?;

                string_range.end().wrapping_add(1)
            }

            OpCode::Over16 => {
                let mut fx = stack_effect!(machine; a:u16, _b0:u16 => _a:u16, _b:u16, a_copy:u16)?;

                fx.a_copy(fx.a());
                fx.commit();

                address + 1
            }

            OpCode::Over32 => {
                let mut fx = stack_effect!(machine; a:u32, _b0:u32 => _a:u32, _b:u32, a_copy:u32)?;

                fx.a_copy(fx.a());
                fx.commit();

                address + 1
            }

            OpCode::Swap16 => {
                let mut fx = stack_effect!(machine; a:u16, b: u16 => b_:u16, a_:u16)?;
                let (a, b) = (fx.a(), fx.b());
                fx.a_(a);
                fx.b_(b);
                fx.commit();

                address + 1
            }

            OpCode::Swap32 => {
                let mut fx = stack_effect!(machine; a:u32, b: u32 => b_:u32, a_:u32)?;
                let (a, b) = (fx.a(), fx.b());
                fx.a_(a);
                fx.b_(b);
                fx.commit();

                address + 1
            }

            OpCode::Dup16 => {
                let mut fx = stack_effect!(machine; x:u16 => _x:u16, x_copy:u16)?;
                fx.x_copy(fx.x());
                fx.commit();

                address + 1
            }

            OpCode::Dup32 => {
                let mut fx = stack_effect!(machine; x:u32 => _x:u32, x_copy:u32)?;
                fx.x_copy(fx.x());
                fx.commit();

                address + 1
            }

            OpCode::Drop16 => {
                machine.memory.data_pop_u16()?;

                address + 1
            }

            OpCode::Add16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;

                fx.c(fx.a().wrapping_add(fx.b()));
                fx.commit();

                address + 1
            }

            OpCode::Sub16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;

                fx.c(fx.a().wrapping_sub(fx.b()));
                fx.commit();

                address + 1
            }

            OpCode::Mul16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;

                fx.c(fx.a().wrapping_mul(fx.b()));
                fx.commit();

                address + 1
            }

            OpCode::Div16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;

                fx.c(fx.a().wrapping_div(fx.b()));
                fx.commit();

                address + 1
            }

            OpCode::Load8 => {
                let mut fx = stack_effect!(machine; address:Address => value:u16)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address,
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                fx.value(fx.machine.memory.raw_memory.read_u8(target_address) as u16);
                fx.commit();

                address + 1
            }

            OpCode::Store8 => {
                let fx = stack_effect!(machine; value: u8, address: Address =>)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address,
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                fx.machine.memory.raw_memory.write_u8(target_address, fx.value());

                fx.commit();

                address + 1
            }

            OpCode::Load16 => {
                let mut fx = stack_effect!(machine; address:Address => value:u16)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address.wrapping_add(1),
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                fx.value(unsafe { fx.machine.memory.raw_memory.read_u16(target_address) });
                fx.commit();

                address + 1
            }

            OpCode::Store16 => {
                let fx = stack_effect!(machine; value:u16, address: Address =>)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address.wrapping_add(1),
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                unsafe { fx.machine.memory.raw_memory.write_u16(target_address, fx.value()) };
                fx.commit();

                address + 1
            }

            OpCode::Load32 => {
                let mut fx = stack_effect!(machine; address:Address => value:u32)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address.wrapping_add(3),
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                fx.value(unsafe { fx.machine.memory.raw_memory.read_u32(target_address) });
                fx.commit();

                address + 1
            }

            OpCode::Store32 => {
                let fx = stack_effect!(machine; value:u32, address: Address =>)?;
                let target_address = fx.address();

                fx.machine.memory.raw_memory.validate_access(
                    target_address..=target_address.wrapping_add(3),
                    fx.machine.memory.raw_memory.address_range(),
                )?;

                unsafe { fx.machine.memory.raw_memory.write_u32(target_address, fx.value()) };

                fx.commit();

                address + 1
            }

            OpCode::Invert16 => {
                let mut fx = stack_effect!(machine; a:u16 => b:u16)?;
                fx.b(!fx.a());
                fx.commit();

                address + 1
            }

            OpCode::And16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;
                fx.c(fx.a() & fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Or16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;
                fx.c(fx.a() | fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Xor16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => c:u16)?;
                fx.c(fx.a() ^ fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Eq16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16 => r:bool)?;
                fx.r(fx.a() == fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Lt16 => {
                let mut fx = stack_effect!(machine; a:i16, b:i16 => r:bool)?;
                fx.r(fx.a() < fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Gt16 => {
                let mut fx = stack_effect!(machine; a:i16, b:i16 => r:bool)?;
                fx.r(fx.a() > fx.b());
                fx.commit();

                address + 1
            }

            OpCode::Emit => {
                let char_code = machine.memory.data_pop_u16()?;

                machine.output.putc(char_code)?;

                address + 1
            }
        })
    }

    pub fn format_at(writer: &mut impl io::Write, machine: &Machine, address: Address) -> Result<Address, io::Error> {
        let op_code = machine.memory.raw_memory.read_u8(address);

        write!(writer, "{:04X}: ", address)?;

        match OpCode::from_int(op_code) {
            Err(_) => {
                writeln!(writer, "(illegal op-code = {})", op_code)?;
                Ok(address + 1)
            }
            Ok(op) => op.format(writer, machine, address)
        }
    }

    pub fn format(self, writer: &mut impl io::Write, machine: &Machine, address: Address) -> Result<Address, io::Error> {
        fn trivial(writer: &mut impl io::Write, address: Address, name: &str) -> Result<Address, io::Error> {
            writeln!(writer, "{}", name)?;
            Ok(address + 1)
        }

        Ok(match self {
            OpCode::Noop => trivial(writer, address, "noop")?,
            OpCode::DefaultArticleStart => trivial(writer, address, "start_article")?,
            OpCode::Return => trivial(writer, address, "ret")?,
            OpCode::Call => {
                let call_address = unsafe { machine.memory.raw_memory.read_u16(address + 1) };
                writeln!(writer, "call {:04X}", call_address)?;
                address + 3
            }
            OpCode::Literal16 => {
                let value = unsafe { machine.memory.raw_memory.read_u16(address + 1) };
                writeln!(writer, "push16 {:04X} ({}, {})", value, value, value as i16)?;
                address + 3
            }
            OpCode::LiteralString => {
                let (range, content) = match ReadableSizedString::new(&machine.memory.raw_memory, address + 1, machine.memory.get_used_dict_segment()) {
                    Ok(s) => (s.full_range(), s.as_bytes()),
                    Err(_) => (address + 1..=address + 1, b"<<<<invalid string>>>>".as_slice())
                };

                match from_utf8(content) {
                    Ok(s) => writeln!(writer, "pushStr {}", s)?,
                    Err(_) => writeln!(writer, "pushStr {:?}", content)?
                }

                range.end().wrapping_add(1)
            }
            OpCode::GoTo => {
                let call_address = unsafe { machine.memory.raw_memory.read_u16(address + 1) };
                writeln!(writer, "jump {:04X}", call_address)?;
                address + 3
            }
            OpCode::GoToIfZ => {
                let call_address = unsafe { machine.memory.raw_memory.read_u16(address + 1) };
                writeln!(writer, "jumpz {:04X}", call_address)?;
                address + 3
            }
            OpCode::ExecBuiltin => {
                let (range, content) = match ReadableSizedString::new(&machine.memory.raw_memory, address + 1, machine.memory.get_used_dict_segment()) {
                    Ok(s) => (s.full_range(), s.as_bytes()),
                    Err(_) => (address + 1..=address + 1, b"<<<<invalid string>>>>".as_slice())
                };

                match from_utf8(content) {
                    Ok(s) => writeln!(writer, "execBuiltin {}", s)?,
                    Err(_) => writeln!(writer, "execBuiltin {:?}", content)?
                }

                range.end().wrapping_add(1)
            }
            OpCode::Dup32 => trivial(writer, address, "dup32")?,
            OpCode::Over16 => trivial(writer, address, "over")?,
            OpCode::Over32 => trivial(writer, address, "over32")?,
            OpCode::Swap16 => trivial(writer, address, "swap")?,
            OpCode::Swap32 => trivial(writer, address, "swap32")?,
            OpCode::Dup16 => trivial(writer, address, "dup")?,
            OpCode::Add16 => trivial(writer, address, "add")?,
            OpCode::Sub16 => trivial(writer, address, "sub")?,
            OpCode::Mul16 => trivial(writer, address, "mul")?,
            OpCode::Div16 => trivial(writer, address, "div")?,
            OpCode::Load16 => trivial(writer, address, "load")?,
            OpCode::Store16 => trivial(writer, address, "store")?,
            OpCode::Load8 => trivial(writer, address, "load8")?,
            OpCode::Store8 => trivial(writer, address, "store8")?,
            OpCode::Load32 => trivial(writer, address, "load32")?,
            OpCode::Store32 => trivial(writer, address, "store32")?,
            OpCode::Drop16 => trivial(writer, address, "drop")?,
            OpCode::Invert16 => trivial(writer, address, "invert")?,
            OpCode::And16 => trivial(writer, address, "and")?,
            OpCode::Or16 => trivial(writer, address, "or")?,
            OpCode::Xor16 => trivial(writer, address, "xor")?,
            OpCode::Eq16 => trivial(writer, address, "eq")?,
            OpCode::Lt16 => trivial(writer, address, "lt")?,
            OpCode::Gt16 => trivial(writer, address, "gt")?,
            OpCode::Emit => trivial(writer, address, "emit")?,
        })
    }
}
