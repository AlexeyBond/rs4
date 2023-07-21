use std::io;
use std::io::{Error as IOError, Stdin, stdin, Write};

use crate::input::InputError::BufferOverflow;

#[derive(Debug)]
pub enum InputError {
    StdIOError(IOError),
    IllegalOffset,
    BufferOverflow,
}

impl From<IOError> for InputError {
    fn from(err: IOError) -> Self {
        InputError::StdIOError(err)
    }
}

fn is_whitespace(chr: u8) -> bool {
    chr.is_ascii_whitespace()
}

pub trait Input {
    fn read(&mut self) -> Result<Option<u8>, InputError>;

    fn tell(&self) -> Result<u32, InputError>;

    fn seek(&mut self, offset: u32) -> Result<(), InputError>;

    fn read_word<'a, 'b>(&'a mut self, buffer: &'b mut [u8]) -> Result<&'b [u8], InputError> {
        let mut read_len: usize;

        loop {
            match self.read()? {
                None => { return Ok(&buffer[0..0]); }
                Some(chr) if !is_whitespace(chr) => {
                    read_len = 1;
                    buffer[0] = chr;
                    break;
                }
                _ => { continue; }
            }
        }

        loop {
            match self.read()? {
                None => {
                    return Ok(&buffer[0..read_len]);
                }
                Some(chr) if is_whitespace(chr) => {
                    return Ok(&buffer[0..read_len]);
                }
                Some(chr) => {
                    if read_len >= buffer.len() {
                        return Err(BufferOverflow);
                    }

                    buffer[read_len] = chr;
                    read_len += 1;
                }
            }
        }
    }
}

pub struct EmptyInput {}

impl Input for EmptyInput {
    fn read(&mut self) -> Result<Option<u8>, InputError> {
        Ok(None)
    }

    fn tell(&self) -> Result<u32, InputError> {
        Ok(0)
    }

    fn seek(&mut self, offset: u32) -> Result<(), InputError> {
        if offset != 0 {
            Err(InputError::IllegalOffset)
        } else {
            Ok(())
        }
    }
}

pub struct StaticStringInput {
    text: &'static str,
    offset: u32,
}

impl Default for StaticStringInput {
    fn default() -> Self {
        Self::new("")
    }
}

impl StaticStringInput {
    pub fn new(text: &'static str) -> StaticStringInput {
        StaticStringInput {
            text,
            offset: 0,
        }
    }
}

impl Input for StaticStringInput {
    fn read(&mut self) -> Result<Option<u8>, InputError> {
        let b_str = self.text.as_bytes();
        let offset = self.offset as usize;

        if offset < b_str.len() {
            self.offset += 1;

            Ok(Some(b_str[offset]))
        } else {
            Ok(None)
        }
    }

    fn tell(&self) -> Result<u32, InputError> {
        Ok(self.offset)
    }

    fn seek(&mut self, offset: u32) -> Result<(), InputError> {
        if (offset as usize) >= self.text.len() {
            return Err(InputError::IllegalOffset);
        }

        self.offset = offset;

        Ok(())
    }
}

pub struct StdinInput {
    stdin: Stdin,
    buffer: String,
    offset: u32,
    prompt: Option<String>,
}

impl StdinInput {
    pub fn new() -> StdinInput {
        StdinInput {
            stdin: stdin(),
            buffer: String::new(),
            offset: 0,
            prompt: Some("\n> ".to_string()),
        }
    }
}

impl Default for StdinInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Input for StdinInput {
    fn read(&mut self) -> Result<Option<u8>, InputError> {
        let offset = self.offset as usize;

        if self.buffer.as_bytes().len() <= offset {
            if let Some(prompt) = self.prompt.as_ref() {
                print!("{}", prompt);
                io::stdout().flush()?;
            }

            self.stdin.read_line(&mut self.buffer)?;

            if self.buffer.as_bytes().len() <= offset {
                return Ok(None);
            }
        }

        self.offset += 1;

        Ok(Some(self.buffer.as_bytes()[offset]))
    }

    fn tell(&self) -> Result<u32, InputError> {
        Ok(self.offset)
    }

    fn seek(&mut self, offset: u32) -> Result<(), InputError> {
        if (offset as usize) > self.buffer.as_bytes().len() {
            return Err(InputError::IllegalOffset);
        }

        self.offset = offset;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_string_input_read() {
        let mut input = StaticStringInput::new("foo bar");

        assert_eq!(input.read().unwrap(), Some('f' as u8));
        assert_eq!(input.read().unwrap(), Some('o' as u8));
        assert_eq!(input.read().unwrap(), Some('o' as u8));
        assert_eq!(input.read().unwrap(), Some(' ' as u8));
        assert_eq!(input.read().unwrap(), Some('b' as u8));
        assert_eq!(input.read().unwrap(), Some('a' as u8));
        assert_eq!(input.read().unwrap(), Some('r' as u8));
        assert_eq!(input.read().unwrap(), None);
    }

    #[test]
    fn test_string_input_read_words() {
        let mut buf = [0u8; 10];
        let mut input = StaticStringInput::new("foo\nbar   baz");

        assert_eq!(input.read_word(&mut buf).unwrap(), "foo".as_bytes());
        assert_eq!(input.read_word(&mut buf).unwrap(), "bar".as_bytes());
        assert_eq!(input.read_word(&mut buf).unwrap(), "baz".as_bytes());
        assert_eq!(input.read_word(&mut buf).unwrap(), "".as_bytes());
    }

    #[test]
    fn test_string_input_tell() {
        let mut input = StaticStringInput::new("foo bar");

        assert_eq!(input.tell().unwrap(), 0);

        input.read().unwrap();
        input.read().unwrap();
        input.read().unwrap();

        assert_eq!(input.tell().unwrap(), 3);
    }

    #[test]
    fn test_string_input_seek() {
        let mut input = StaticStringInput::new("foo bar");

        input.seek(5).unwrap();
        assert_eq!(input.read().unwrap(), Some('a' as u8));

        input.seek(0).unwrap();
        assert_eq!(input.read().unwrap(), Some('f' as u8));

        let bad_seek_result = input.seek(10);
        assert!(matches!(bad_seek_result, Err(InputError::IllegalOffset)))
    }
}
