use crate::input::StaticStringInput;
use crate::machine::{Machine, MachineExtensions};
use crate::machine_error::MachineError;
use crate::machine_memory::MachineMemory;
use crate::output::StringOutput;

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

#[derive(Default)]
pub struct TestMachineExtensions {
    pub input: StaticStringInput,
    pub output: StringOutput,
}

impl MachineExtensions for TestMachineExtensions {
    type TInput = StaticStringInput;
    type TOutput = StringOutput;

    fn get_input(&mut self) -> &mut Self::TInput {
        &mut self.input
    }

    fn get_output(&mut self) -> &mut Self::TOutput {
        &mut self.output
    }
}

pub type TestMachine = Machine<TestMachineExtensions>;

pub struct TestRunResult {
    pub machine: TestMachine,
    pub result: Result<(), MachineError>,
}

impl TestMachine {
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

    pub fn run_with_test_input(input_text: &'static str) -> TestRunResult {
        let mut machine = TestMachine::default();

        machine.extensions.input = StaticStringInput::new(input_text);

        let result = machine.interpret_input();

        TestRunResult {
            machine,
            result,
        }
    }
}
