use std::fs;
use std::io::{stdout, Write};

use rs4::input::StdinInput;
use rs4::machine::{Machine, MachineExtensions};
use rs4::output::StdoutOutput;

#[derive(Default)]
struct InteractiveMachineExtensions {
    i: StdinInput,
    o: StdoutOutput,
}

impl MachineExtensions for InteractiveMachineExtensions {
    type TInput = StdinInput;
    type TOutput = StdoutOutput;

    fn get_input(&mut self) -> &mut Self::TInput {
        &mut self.i
    }

    fn get_output(&mut self) -> &mut Self::TOutput {
        &mut self.o
    }
}

fn main() {
    let mut machine = Machine::<InteractiveMachineExtensions>::default();

    loop {
        match machine.interpret_input() {
            Ok(_) => { return; }
            Err(err) => {
                print!("Error: ");
                err.pretty_print(&mut stdout(), &machine).unwrap();
                print!("\n-----\nMachine state:\n");
                machine.print_state(&mut stdout()).unwrap();
                machine.print_disassembly(&mut stdout()).unwrap();

                stdout().flush().unwrap();

                machine.memory.raw_memory.dump_to(&mut fs::File::create("./dump.bin").unwrap()).unwrap();
            }
        };
    }
}
