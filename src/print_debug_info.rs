use std::cmp::min;
use std::io;
use std::str::from_utf8;

use crate::machine::Machine;
use crate::machine_memory::MachineMemory;
use crate::mem::Address;

const MAX_STACK_ENTRIES_TO_PRINT: u16 = 16;

impl MachineMemory {
    fn print_stack_state(&self, f: &mut impl io::Write, sp: Address, depth: u16) -> io::Result<()> {
        write!(f, "\t")?;

        if depth == 0 {
            write!(f, "(empty)\n")?;
        } else {
            let entries_to_print = min(MAX_STACK_ENTRIES_TO_PRINT, depth);

            if entries_to_print < depth {
                write!(f, "..., ")?;
            }

            for i in (0..entries_to_print).rev() {
                let value = unsafe { self.raw_memory.read_u16(sp + 2 * i) };

                write!(f, "{value:04X} ({value:>5}){}", if i == 0 { "\n" } else { ", " })?;
            }
        }

        Ok(())
    }

    fn print_articles(&self, f: &mut impl io::Write) -> io::Result<()> {
        let article_count = self.articles().count();

        if article_count == 0 {
            write!(f, "No valid articles.\n")?;
            return Ok(());
        }

        write!(f, "Article(s) ({article_count}):\n\t")?;

        for article in self.articles() {
            write!(f, "{}, ", from_utf8(article.name().as_bytes()).unwrap_or("(not printable)"))?;
        }

        write!(f, "\n")?;

        Ok(())
    }

    pub fn print_memory_state(&self, f: &mut impl io::Write) -> io::Result<()> {
        let data_stack_depth = self.data_stack_depth();
        write!(f, "Data stack (depth: {data_stack_depth}):\n")?;
        self.print_stack_state(f, self.data_stack_ptr, data_stack_depth)?;

        let call_stack_depth = self.call_stack_depth();
        write!(f, "Call stack (depth: {call_stack_depth}):\n")?;
        self.print_stack_state(f, self.call_stack_ptr, call_stack_depth)?;

        write!(f, "Dictionary size: {} byte(s)\n", self.dictionary_size())?;

        self.print_articles(f)?;

        Ok(())
    }
}

impl Machine {
    pub fn print_state(&self, f: &mut impl io::Write) -> io::Result<()> {
        self.memory.print_memory_state(f)?;

        write!(f, "Mode: {}\n", self.mode)?;
        match self.input.tell() {
            Ok(position) => write!(f, "Input position: {position}\n"),
            Err(err) => write!(f, "Input broken: {err:?}\n"),
        }?;

        Ok(())
    }
}
