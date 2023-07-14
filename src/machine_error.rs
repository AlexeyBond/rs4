use std::io;
use std::str::from_utf8;

use crate::input::InputError;
use crate::machine::{Machine, MachineMode};
use crate::mem::{Address, MemoryAccessError};
use crate::output::OutputError;
use crate::sized_string::ReadableSizedString;

#[derive(Debug)]
pub enum MachineError {
    MemoryAccessError(MemoryAccessError),
    InputError(InputError),
    UnexpectedInputEOF,
    OutputError(OutputError),
    IllegalOpCodeError {
        address: Address,
        op_code: u8,
    },
    IllegalWord(Option<Address>),
    NoArticle,
    UnexpectedArticleType,
    IllegalMode {
        expected: MachineMode,
        actual: MachineMode,
    },
    Exited,
}

impl From<MemoryAccessError> for MachineError {
    fn from(err: MemoryAccessError) -> Self {
        MachineError::MemoryAccessError(err)
    }
}

impl From<InputError> for MachineError {
    fn from(err: InputError) -> Self {
        MachineError::InputError(err)
    }
}

impl From<OutputError> for MachineError {
    fn from(err: OutputError) -> Self {
        MachineError::OutputError(err)
    }
}

impl MachineError {
    pub fn pretty_print(&self, f: &mut impl io::Write, machine: &Machine) -> io::Result<()> {
        match self {
            MachineError::InputError(input_err) => {
                match input_err {
                    InputError::StdIOError(err) => {
                        write!(f, "IO error: {}", err)
                    }
                    InputError::IllegalOffset => {
                        write!(f, "Illegal input offset requested")
                    }
                    InputError::BufferOverflow => {
                        write!(f, "Input buffer overflow")
                    }
                }
            }
            MachineError::OutputError(output_err) => {
                match output_err {
                    OutputError::StdIOError(err) => {
                        write!(f, "IO error: {}", err)
                    }
                }
            }
            MachineError::IllegalWord(Some(word_name_address)) => {
                let name_bytes = ReadableSizedString::new(&machine.memory.raw_memory, *word_name_address, machine.memory.raw_memory.address_range())
                    .unwrap()
                    .as_bytes();

                write!(f, "Illegal word: {}", from_utf8(name_bytes).unwrap_or("(unprintable name)"))
            }
            MachineError::MemoryAccessError(MemoryAccessError { access_range, segment }) => {
                write!(f, "Illegal memory access attempt to {} byte(s) at {:X?} (allowed range is {:X?})", access_range.len(), access_range, segment)
            }
            _ => {
                write!(f, "{:?}", self)
            }
        }
    }
}
