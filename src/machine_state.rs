use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MachineState {
    Interpreter,
    Compiler,
}

impl Display for MachineState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "{}",
            match self {
                MachineState::Compiler => "compiler",
                MachineState::Interpreter => "interpreter"
            }
        )
    }
}
