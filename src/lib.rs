pub mod mem;
pub mod machine;
mod readable_article;
mod opcodes;
mod input;
mod sized_string;
mod builtin_words;
mod literal;
mod machine_memory;

#[cfg(test)]
mod machine_testing;
