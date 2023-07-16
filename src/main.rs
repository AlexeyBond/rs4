use std::{fs, io};
use std::io::{stdout, Write};

use rs4::input::StdinInput;
use rs4::machine::Machine;

fn dump_memory(machine: &Machine) -> io::Result<()> {
    let mut file = fs::File::create("./dump.bin")?;
    machine.memory.raw_memory.dump_to(&mut file)
}

fn main() {
    let mut machine = Machine::default();

    machine.input = Box::new(StdinInput::new());

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

                dump_memory(&machine).unwrap();
            }
        };
    }
}
