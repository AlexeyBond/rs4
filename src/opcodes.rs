use std::io;
use std::str::from_utf8;
use int_enum::IntEnum;
use crate::builtin_words::process_builtin_word;

use crate::machine::Machine;
use crate::machine_error::MachineError;
use crate::machine_state::MachineState;
use crate::mem::Address;
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
    /// Pushes address and size of that string to data stack.
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

    CallPop16 = 9,
    CallPush16 = 10,
    CallPop32 = 11,
    CallPush32 = 12,
    CallRead16 = 13,
    CallRead32 = 14,

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
    Rot16 = 147,
    I16ToI32 = 148,
    Abs16 = 149,

    Emit = 200,
    PnoInit = 201,
    PnoPut = 202,
    PnoFinish = 203,
    PnoPutDigit = 204,
    EmitString = 205,
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
                match machine.memory.get_state() {
                    MachineState::Interpreter => {
                        address + 1 // Noop
                    }
                    MachineState::Compiler => {
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
                )?.content_range();

                let mut fx = stack_effect!(machine; => address:Address, size:u16)?;
                fx.address(*string_range.start());
                fx.size(string_range.len() as u16);
                fx.commit();

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
            OpCode::Rot16 => {
                let mut fx = stack_effect!(machine; a:u16, b:u16, c:u16 => b1:u16, c1:u16, a1:u16)?;
                let (a, b, c) = (fx.a(), fx.b(), fx.c());
                fx.a1(a);
                fx.b1(b);
                fx.c1(c);
                fx.commit();

                address + 1
            }
            OpCode::I16ToI32 => {
                let mut fx = stack_effect!(machine; a:i16 => b:i32)?;
                fx.b(fx.a() as i32);
                fx.commit();

                address + 1
            }
            OpCode::CallPop16 => {
                let val = machine.memory.call_pop_u16()?;
                machine.memory.data_push_u16(val)?;

                address + 1
            }
            OpCode::CallPush16 => {
                let val = machine.memory.data_pop_u16()?;
                machine.memory.call_push_u16(val)?;

                address + 1
            }
            OpCode::CallPop32 => {
                let val = machine.memory.call_pop_u32()?;
                machine.memory.data_push_u32(val)?;

                address + 1
            }
            OpCode::CallPush32 => {
                let val = machine.memory.data_pop_u32()?;
                machine.memory.call_push_u32(val)?;

                address + 1
            }
            OpCode::CallRead16 => {
                let val = machine.memory.call_get_u16()?;
                machine.memory.data_push_u16(val)?;

                address + 1
            }
            OpCode::CallRead32 => {
                let val = machine.memory.call_get_u32()?;
                machine.memory.data_push_u32(val)?;

                address + 1
            }
            OpCode::Abs16 => {
                let mut fx = stack_effect!(machine; a:i16 => b:i16)?;
                fx.b(fx.a().abs());
                fx.commit();

                address + 1
            }
            OpCode::PnoInit => {
                machine.memory.clear_pno_buffer();

                address + 1
            }
            OpCode::PnoPut => {
                let ch = machine.memory.data_pop_u16()? as u8;
                machine.memory.pno_put(ch)?;

                address + 1
            }
            OpCode::PnoFinish => {
                let (addr, size) = machine.memory.pno_finish();
                let mut fx = stack_effect!(machine; _x:u32 => address:Address, size:u16)?;
                fx.address(addr);
                fx.size(size as u16);
                fx.commit();

                address + 1
            }
            OpCode::PnoPutDigit => {
                let mut fx = stack_effect!(machine; i:u32 => o:u32)?;
                let base = fx.machine.memory.get_base() as u32;
                let i = fx.i();

                let digit = (i % base) as u8;
                fx.o(i / base);

                fx.commit();

                let digit_char = if digit < 10 {
                    b'0'.wrapping_add(digit)
                } else {
                    b'A'.wrapping_add(digit).wrapping_sub(10)
                };

                machine.memory.pno_put(digit_char)?;

                address + 1
            }
            OpCode::EmitString => {
                let fx = stack_effect!(machine; addr: Address, size: u16 => )?;
                let (addr, size) = (fx.addr(), fx.size());
                fx.commit();

                let text = machine.memory.raw_memory.address_slice(addr, size as usize);

                machine.output.puts(text)?;

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
            OpCode::Rot16 => trivial(writer, address, "rot")?,
            OpCode::I16ToI32 => trivial(writer, address, "s>d")?,
            OpCode::CallPop16 => trivial(writer, address, "call_pop")?,
            OpCode::CallPush16 => trivial(writer, address, "call_push")?,
            OpCode::CallPop32 => trivial(writer, address, "call_pop32")?,
            OpCode::CallPush32 => trivial(writer, address, "call_push32")?,
            OpCode::CallRead16 => trivial(writer, address, "call_get")?,
            OpCode::CallRead32 => trivial(writer, address, "call_get32")?,
            OpCode::Abs16 => trivial(writer, address, "abs")?,
            OpCode::Emit => trivial(writer, address, "emit")?,
            OpCode::PnoInit => trivial(writer, address, "pno:init")?,
            OpCode::PnoPut => trivial(writer, address, "pno:put")?,
            OpCode::PnoFinish => trivial(writer, address, "pno:finish")?,
            OpCode::PnoPutDigit => trivial(writer, address, "pno:put_digit")?,
            OpCode::EmitString => trivial(writer, address, "emit_str")?,
        })
    }
}
