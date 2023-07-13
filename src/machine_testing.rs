use crate::input::StaticStringInput;
use crate::machine::{Machine, MachineError};
use crate::machine_memory::MachineMemory;

pub enum StackElement {
    Cell(u16),
    DoubleCell(u32),
}

impl StackElement {
    fn size(&self) -> u16 {
        match self {
            StackElement::Cell(_) => 1,
            StackElement::DoubleCell(_) => 2,
        }
    }

    fn assert(&self, mem: &mut MachineMemory) {
        match self {
            StackElement::Cell(value) => {
                assert_eq!(mem.data_pop_u16().unwrap(), *value)
            }

            StackElement::DoubleCell(value) => {
                assert_eq!(mem.data_pop_u32().unwrap(), *value)
            }
        }
    }
}

impl Machine {
    //noinspection RsAssertEqual
    pub fn assert_data_stack_state(&mut self, elements: &[StackElement]) {
        let expected_stack_depth = elements.iter().fold(0, |acc, el| acc + el.size());
        assert!(
            self.memory.data_stack_depth() == expected_stack_depth,
            "unexpected stack depth: expected {} cells, found {}",
            expected_stack_depth, self.memory.data_stack_depth()
        );

        for el in elements.iter().rev() {
            el.assert(&mut self.memory);
        }
    }

    pub fn run_with_test_input(input_text: &'static str) -> Result<Machine, MachineError> {
        let mut m = Machine::default();

        m.input = Box::new(StaticStringInput::new(input_text));
        m.interpret_input()?;

        Ok(m)
    }
}
