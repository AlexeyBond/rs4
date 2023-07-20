use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

use crate::builtin_words::process_builtin_word;
use crate::input::{EmptyInput, Input};
use crate::machine_error::MachineError;
use crate::machine_memory::MachineMemory;
use crate::mem::Address;
use crate::opcodes::OpCode;
use crate::output::{Output, StdoutOutput};

type Result<T> = StdResult<T, MachineError>;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum MachineMode {
    Interpreter,
    Compiler,
}

impl Display for MachineMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "{}",
            match self {
                MachineMode::Compiler => "compiler",
                MachineMode::Interpreter => "interpreter"
            }
        )
    }
}

pub type WordFallbackHandler = fn(machine: &mut Machine, name_address: Address) -> Result<()>;

pub fn default_fallback_handler(_machine: &mut Machine, name_address: Address) -> Result<()> {
    Err(MachineError::IllegalWord(Some(name_address)))
}

pub struct Machine {
    pub input: Box<dyn Input>,

    pub output: Box<dyn Output>,

    pub mode: MachineMode,

    pub word_fallback_handler: WordFallbackHandler,

    pub memory: MachineMemory,
}

impl Machine {
    pub fn reset(&mut self) {
        self.memory.reset();
        self.mode = MachineMode::Interpreter;
    }

    pub fn expect_mode(&self, mode: MachineMode) -> Result<()> {
        if self.mode != mode {
            return Err(MachineError::IllegalMode {
                expected: mode,
                actual: self.mode.clone(),
            });
        }

        Ok(())
    }

    pub fn run_forever(&mut self, start_address: Address) -> Result<()> {
        let mut address = start_address;

        loop {
            address = OpCode::execute_at(self, address)?;
        }
    }

    pub fn run_until_exit(&mut self, start_address: Address) -> Result<()> {
        match self.run_forever(start_address) {
            Err(MachineError::Exited) => Ok(()),
            res => res
        }
    }

    pub fn execute_word(&mut self, name_address: Address) -> Result<()> {
        if let Some(article) = self.memory.lookup_article_name_buf(name_address)? {
            self.run_until_exit(article.body_address())
        } else {
            process_builtin_word(self, name_address)
        }
    }

    pub fn read_input_word(&mut self) -> Result<Option<Address>> {
        Ok(self.memory.read_input_word(self.input.as_mut())?)
    }

    pub fn interpret_input(&mut self) -> Result<()> {
        loop {
            if let Some(name_address) = self.read_input_word()? {
                self.execute_word(name_address)?;
            } else {
                return Ok(());
            }
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine {
            input: Box::new(EmptyInput {}),
            output: Box::new(StdoutOutput::new()),
            mode: MachineMode::Interpreter,
            word_fallback_handler: default_fallback_handler,
            memory: MachineMemory::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::str::from_utf8;
    use crate::machine_testing::*;

    use super::*;

    fn test_16_bit_results(input: &'static str, results: &[u16]) {
        let mut r = Machine::run_with_test_input(input);

        match r.result {
            Ok(_) => {}
            Err(err) => {
                let mut buf = Vec::new();
                err.pretty_print(&mut buf, &r.machine).unwrap();

                panic!("Machine error occurred: {}", from_utf8(buf.as_slice()).unwrap());
            }
        }
        r.machine.assert_data_stack_state(&results.iter().map(|r| StackElement::Cell(*r)).collect::<Vec<_>>())
    }

    #[test]
    fn test_push_literal() {
        test_16_bit_results("1 2", &[1, 2]);
    }

    #[test]
    fn test_arithmetic() {
        test_16_bit_results("1 2 +", &[3]);
        test_16_bit_results("1 -3 -", &[4]);
        test_16_bit_results("42 2 *", &[84]);
        test_16_bit_results("10 2 /", &[5]);
    }

    #[test]
    fn test_store_load_16() {
        test_16_bit_results("42 101 !", &[]);
        test_16_bit_results("42 101 ! 101 @", &[42]);
    }

    #[test]
    fn test_store_load_8() {
        test_16_bit_results("$FFFF 101 C! $FEFE 102 C!", &[]);
        test_16_bit_results("$FFFF 101 C! $FEFE 102 C! 101 C@ 102 C@", &[0xff, 0xfe]);
    }

    #[test]
    fn test_radix_change() {
        test_16_bit_results("100 36 BASE ! zZz", &[100, 46655]);
    }

    fn test_output(input: &'static str, expected_output: &'static [u8]) {
        let result = Machine::run_with_test_input(input);
        let out_vec = (*result.output).borrow();

        assert_eq!(out_vec.as_slice(), expected_output)
    }

    #[test]
    fn test_emit_single_characters() {
        test_output(
            "70 EMIT 79 DUP EMIT EMIT 66 EMIT 65 EMIT 82 EMIT",
            b"FOOBAR",
        )
    }

    #[test]
    fn test_colon_definition() {
        test_16_bit_results(
            ": foo + ;",
            &[],
        );
        test_16_bit_results(
            ": foo + ; 100 1 foo",
            &[101],
        )
    }

    #[test]
    fn test_colon_definition_use() {
        test_16_bit_results(
            ": +3 3 + ; 2 +3 +3",
            &[8],
        )
    }

    #[test]
    fn test_comparison() {
        test_16_bit_results(
            "0 1 < -1 0 < 0 0 < 2 1 <",
            &[0xffff, 0xffff, 0, 0],
        );
        test_16_bit_results(
            "0 1 > -1 0 > 0 0 > 2 1 >",
            &[0, 0, 0, 0xffff],
        );
        test_16_bit_results(
            "0 1 = -1 0 = 0 0 = 2 1 =",
            &[0, 0, 0xffff, 0],
        );
    }

    #[test]
    fn test_logic() {
        test_16_bit_results(
            "TRUE FALSE",
            &[0xffff, 0],
        );
        test_16_bit_results(
            "TRUE FALSE AND FALSE TRUE AND FALSE FALSE AND TRUE TRUE AND",
            &[0, 0, 0, 0xffff],
        );
        test_16_bit_results(
            "TRUE FALSE OR FALSE TRUE OR FALSE FALSE OR TRUE TRUE OR",
            &[0xffff, 0xffff, 0, 0xffff],
        );
        test_16_bit_results(
            "TRUE FALSE XOR FALSE TRUE XOR FALSE FALSE XOR TRUE TRUE XOR",
            &[0xffff, 0xffff, 0, 0],
        );
        test_16_bit_results(
            "TRUE INVERT FALSE INVERT",
            &[0, 0xffff],
        );
    }

    #[test]
    fn test_dup() {
        test_16_bit_results(
            "1 2 DUP",
            &[1, 2, 2],
        );

        test_16_bit_results(
            "3 4 2DUP",
            &[3, 4, 3, 4],
        );
    }

    #[test]
    fn test_drop() {
        test_16_bit_results(
            "1 2 3 DROP",
            &[1, 2],
        );

        test_16_bit_results(
            "4 5 6 2DROP",
            &[4],
        );
    }

    #[test]
    fn test_rot() {
        test_16_bit_results(
            "1 2 3 ROT",
            &[2, 3, 1],
        )
    }

    #[test]
    fn test_immediate() {
        test_16_bit_results(
            "
            : C,, HERE @ C! HERE @ 1 + HERE ! ;
            : ,, HERE @ ! HERE @ 2 + HERE ! ;
            : iff    7 ( OpCode: GoToIfZ ) C,, HERE @ 0 ,, ; IMMEDIATE
            : elsse  6 ( OpCode: GoTo    ) C,, HERE @ 0 ,, SWAP HERE @ SWAP ! ; IMMEDIATE
            : endiff                                            HERE @ SWAP ! ; IMMEDIATE
            : tst 0 < iff -1 elsse 1 endiff ;

            0 tst -1 tst
            ",
            &[1, 0xffff],
        )
    }

    #[test]
    fn test_conditions() {
        test_16_bit_results(
            "
            : myabs 1 SWAP 0 < IF DROP -1 THEN ;

            0 myabs -1 myabs
            ",
            &[1, 0xffff],
        );
    }

    #[test]
    fn test_conditions_2() {
        test_16_bit_results(
            "
            : myabs 0 < IF -1 ELSE 1 THEN ;

            0 myabs -1 myabs
            ",
            &[1, 0xffff],
        );
    }

    #[test]
    fn test_while_loop() {
        test_16_bit_results(
            "
            : 1- 1 - ;
            : FACTORIAL ( +n1 -- +n2 )
               DUP 2 < IF DROP 1 EXIT THEN
               DUP
               BEGIN DUP 2 > WHILE
               1- SWAP OVER * SWAP
               REPEAT DROP
            ;
            8 FACTORIAL
            ",
            &[40320],
        );
    }

    #[test]
    fn test_postpone() {
        test_16_bit_results(
            "
            : endif POSTPONE THEN ; IMMEDIATE
            : myabs 1 SWAP 0 < IF DROP -1 endif ;

            0 myabs -1 myabs
            ",
            &[1, 0xffff],
        )
    }

    #[test]
    fn test_recurse() {
        test_16_bit_results(
            "
            : 1- 1 - ;
            : FACTORIAL ( +n1 -- +n2)
               DUP 2 < IF DROP 1 EXIT THEN
               DUP 1- RECURSE *
            ;
            8 FACTORIAL
            ",
            &[40320],
        )
    }

    #[test]
    fn test_print_string() {
        test_output(
            "
            : say-bye .\" Goodbye world\" ;
            .\" Hello world\" 10 EMIT
            say-bye
            ",
            b"Hello world\nGoodbye world",
        )
    }

    #[test]
    fn test_pictured_number_output() {
        test_output(
            "
            666 S>D <# # # # # #>
            ",
            b"",
        );
        test_output(
            "
            666 S>D <# # # # # #>
            TYPE
            ",
            b"0666",
        );
        test_output(
            "
            1638 16 BASE ! S>D <# # # # # #>
            TYPE
            ",
            b"0666",
        );
    }

    #[test]
    fn test_mode_switch_and_literals() {
        test_16_bit_results(
            ": foo [ 1 2 + ] LITERAL + ;",
            &[],
        );
        test_16_bit_results(
            ": foo [ 1 2 + ] LITERAL + ; 3 foo",
            &[6],
        );
    }
}
