use std::cmp::min;

use crate::mem::{Address, AddressRange, Mem, MemoryAccessError};

pub trait StackEffect {
    /// Size of data popped from stack, in 16-bit words
    fn in_words(&self) -> u16;

    /// Size of data pushed to stack, in 16-bit words
    fn out_words(&self) -> u16;

    /// Address of the highest byte touched by this stack effect with given stack pointer
    fn max_ptr(&self, base: Address) -> Address {
        base.wrapping_add(self.in_words().wrapping_mul(2).wrapping_sub(1))
    }

    /// Value of stack pointer after this stack effect is applied
    fn resulting_ptr(&self, base: Address) -> Address {
        self.max_ptr(base).wrapping_sub(self.out_words().wrapping_mul(2)).wrapping_add(1)
    }

    /// Address of the lowest byte touched by this stack effect with given stack pointer
    fn min_ptr(&self, base: Address) -> Address {
        min(self.resulting_ptr(base), base)
    }

    fn validate_access(&self, mem: &Mem, ptr: Address, segment: AddressRange) -> Result<(), MemoryAccessError> {
        mem.validate_access(
            self.min_ptr(ptr)..=self.max_ptr(ptr),
            segment,
        )
    }
}

pub trait Stackable {
    const SIZE_WORDS: u16;

    unsafe fn read(memory: &Mem, address: Address) -> Self;

    unsafe fn write(&self, memory: &mut Mem, address: Address);
}

impl Stackable for u16 {
    const SIZE_WORDS: u16 = 1;

    unsafe fn read(memory: &Mem, address: Address) -> Self {
        memory.read_u16(address)
    }

    unsafe fn write(&self, memory: &mut Mem, address: Address) {
        memory.write_u16(address, *self)
    }
}

macro_rules! count_size {
    () => (0);
    ($t:ty) => (<$t as crate::stack_effect::Stackable>::SIZE_WORDS);
    ($t:ty, $($rest:ty),+) => (count_size!($t) + count_size!($($rest),+));
}

macro_rules! implement_getters {
    () => ();
    ($n:ident : $t:ty) => (implement_getters!($n : $t ,););
    ($n:ident : $t:ty, $($ns:ident : $ts:ty),*) => (
        pub fn $n(&self) -> $t {
            let address = self.machine.memory.data_stack_ptr + (count_size!($($ts),*)) * 2;

            unsafe {
                <$t as crate::stack_effect::Stackable>::read(
                    &self.machine.memory.raw_memory,
                    address,
                )
            }
        }

        implement_getters!($($ns : $ts),*);
    );
}

macro_rules! implement_setters {
    () => ();
    ($n:ident : $t:ty) => (implement_setters!($n : $t ,););
    ($n:ident : $t:ty, $($ns:ident : $ts:ty),*) => (
        pub fn $n(&mut self, value: $t) -> &mut Self {
            use crate::stack_effect::Stackable;

            let address = self.resulting_ptr(self.machine.memory.data_stack_ptr) + (count_size!($($ts),*)) * 2;

            unsafe {
                value.write(
                    &mut self.machine.memory.raw_memory,
                    address,
                )
            }

            self
        }

        implement_setters!($($ns : $ts),*);
    );
}

macro_rules! stack_effect {
    ($machine:expr; $($in_name:ident : $in_type:ty),* => $($out_name:ident : $out_type:ty),*) => ({
        use crate::stack_effect::count_size;
        use crate::stack_effect::implement_getters;
        use crate::stack_effect::implement_setters;
        use crate::stack_effect::StackEffect;
        use crate::mem::MemoryAccessError;

        struct Effect<'m> {
            machine: &'m mut crate::machine::Machine,
        }

        impl <'m>StackEffect for Effect<'m> {
            fn in_words(&self) -> u16 {
                count_size!($($in_type),*)
            }

            fn out_words(&self) -> u16 {
                count_size!($($out_type),*)
            }
        }

        impl <'m>Effect<'m> {
            implement_getters!($($in_name : $in_type),*);
            implement_setters!($($out_name : $out_type),*);

            fn commit(self) {
                self.machine.memory.data_stack_ptr = self.resulting_ptr(self.machine.memory.data_stack_ptr);
            }

            fn validate(self) -> Result<Self, MemoryAccessError> {
                self.validate_access(
                    &self.machine.memory.raw_memory,
                    self.machine.memory.data_stack_ptr,
                    self.machine.memory.get_data_stack_segment(),
                )?;

                Ok(self)
            }
        }

        (Effect { machine: $machine }).validate()
    })
}

#[cfg(test)]
mod test {
    use crate::machine::Machine;
    use crate::machine_testing::StackElement;

    #[test]
    fn test_2_to_1_effect() {
        let mut machine = Machine::default();

        machine.memory.data_push_u16(0x1234).unwrap();
        machine.memory.data_push_u16(0xabcd).unwrap();

        let mut fx = stack_effect!(&mut machine; a:u16, b:u16 => c:u16).unwrap();

        assert_eq!(fx.a(), 0x1234);
        assert_eq!(fx.b(), 0xabcd);

        fx.c(0xef56);

        fx.commit();

        machine.assert_data_stack_state(&[StackElement::Cell(0xef56)]);
    }

    #[test]
    fn test_1_to_2_effect() {
        let mut machine = Machine::default();

        machine.memory.data_push_u16(0x1234).unwrap();

        let mut fx = stack_effect!(&mut machine; a:u16 => b:u16, c:u16).unwrap();

        assert_eq!(fx.a(), 0x1234);

        fx.b(0xef56).c(0x4213);

        fx.commit();

        machine.assert_data_stack_state(&[StackElement::Cell(0xef56), StackElement::Cell(0x4213)]);
    }
}

pub(crate) use count_size;
pub(crate) use implement_getters;
pub(crate) use implement_setters;
pub(crate) use stack_effect;
