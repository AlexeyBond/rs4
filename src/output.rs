use std::cell::RefCell;
use std::io::{Error as IOError, stdout, Stdout, Write};
use std::rc::Rc;

fn word_to_char(word: u16) -> u8 {
    (word & 0xff) as u8
}

#[derive(Debug)]
pub enum OutputError {
    StdIOError(IOError),
}

impl From<IOError> for OutputError {
    fn from(err: IOError) -> Self {
        OutputError::StdIOError(err)
    }
}

pub trait Output {
    fn putc(&mut self, character: u16) -> Result<(), OutputError>;

    fn puts(&mut self, data: &[u8]) -> Result<(), OutputError>;

    fn flush(&mut self) -> Result<(), OutputError>;
}

pub struct StdoutOutput {
    stdout: Stdout,
}

impl StdoutOutput {
    pub fn new() -> StdoutOutput {
        StdoutOutput {
            stdout: stdout()
        }
    }
}

impl Output for StdoutOutput {
    fn putc(&mut self, character: u16) -> Result<(), OutputError> {
        self.stdout.write(&[word_to_char(character)])?;

        Ok(())
    }

    fn puts(&mut self, data: &[u8]) -> Result<(), OutputError> {
        self.stdout.write(data)?;

        Ok(())
    }

    fn flush(&mut self) -> Result<(), OutputError> {
        self.stdout.flush()?;

        Ok(())
    }
}

pub struct StringOutput {
    pub content: Rc<RefCell<Vec<u8>>>,
}

impl StringOutput {
    pub fn new(content: Rc<RefCell<Vec<u8>>>) -> StringOutput {
        StringOutput { content }
    }
}

impl Output for StringOutput {
    fn putc(&mut self, character: u16) -> Result<(), OutputError> {
        (*self.content).borrow_mut().push(word_to_char(character));

        Ok(())
    }

    fn puts(&mut self, data: &[u8]) -> Result<(), OutputError> {
        (*self.content).borrow_mut().extend_from_slice(data);

        Ok(())
    }

    fn flush(&mut self) -> Result<(), OutputError> {
        Ok(())
    }
}
