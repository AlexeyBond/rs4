use crate::machine::{Machine, MachineError};

fn op_push_u16(machine: &mut Machine, value: u16) -> Result<(), MachineError> {
    machine.memory.data_push_u16(value).map_err(|err| err.into())
}

fn op_drop_u16(machine: &mut Machine) -> Result<(), MachineError> {
    machine.memory.data_pop_u16()?;

    Ok(())
}


