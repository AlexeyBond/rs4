use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};
use crate::sized_string::ReadableSizedString;

pub struct ReadableArticle<'m> {
    memory: &'m Mem,
    header_address: Address,
}

// 2 bytes of previous article link, 1 byte of name size
const MIN_HEADER_SIZE: u16 = 3;

/// A helper to access an article stored in machine's dictionary.
///
/// The article is stored as follows:
/// - first 2 bytes contain address of the previous word
/// - next is an article name stored as a sized string -
///   one byte containing string size followed by string's content
/// - following bytes contain article's body
impl<'m> ReadableArticle<'m> {
    pub fn new(memory: &Mem, header_address: Address, safe_memory_range: AddressRange) -> Result<ReadableArticle, MemoryAccessError> {
        memory.validate_access(
            header_address..=header_address + MIN_HEADER_SIZE,
            safe_memory_range.clone(),
        )?;

        let article = ReadableArticle {
            memory,
            header_address,
        };

        article.name().validate_content(safe_memory_range)?;

        Ok(article)
    }

    /// Address of a sized string containing the name of this article.
    pub fn name_address(&self) -> Address {
        self.header_address.wrapping_add(2)
    }

    /// Reference to a slice of machine's memory that contains article name.
    pub fn name(&self) -> ReadableSizedString<'m> {
        let sized_str = unsafe {
            ReadableSizedString::unsafe_new(self.memory, self.name_address())
        };

        return sized_str;
    }

    /// Address of first byte of article body.
    pub fn body_address(&self) -> Address {
        self.name_address().wrapping_add(self.name().read_length() as u16)
    }

    /// Address of header of the previous article
    pub fn previous_address(&self) -> Address {
        unsafe { self.memory.read_u16(self.header_address) }
    }

    /// Previous article represented as a ReadableArticle.
    ///
    /// Returns `None` if this is the first article.
    pub fn previous_article<'a>(&'a self, safe_memory_range: AddressRange) -> Result<Option<ReadableArticle<'m>>, MemoryAccessError> {
        let prev_address = self.previous_address();

        if prev_address >= self.header_address {
            return Ok(None);
        }

        Ok(Some(ReadableArticle::new(self.memory, prev_address, safe_memory_range)?))
    }
}
