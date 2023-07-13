use std::io::{stdout, Write};

use rs4::input::StdinInput;
use rs4::machine::Machine;

fn main() {
    let mut machine = Machine::default();

    machine.input = Box::new(StdinInput::new());

    loop {
        match machine.interpret_input() {
            Ok(_) => { return; }
            Err(err) => {
                let mut msg: String = String::new();
                err.pretty_print(&mut msg, &machine).unwrap();

                println!("{}", msg);

                stdout().flush().unwrap();
            }
        };
    }
}
