use crate::machine::{Machine, MachineError};
use crate::mem::Address;
use crate::memory_segment::{DataSegment, StaticMemorySegment};
use crate::sized_string::ReadableSizedString;

pub fn process_builtin_word(machine: &mut Machine, name_address: Address) -> Result<(), MachineError> {
    match ReadableSizedString::new(&machine.memory, name_address, DataSegment::get_static_range(machine))
        .map_err(|err| MachineError::MemoryAccessError(DataSegment::NAME, err))?
        .as_bytes() {
        b"DROP" => {
            todo!("execute or write opcode")
        }
        _ => (machine.word_fallback_handler)(machine, name_address)
    }?;

    Ok(())
}
