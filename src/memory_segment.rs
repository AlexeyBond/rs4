use crate::machine::Machine;
use crate::mem::AddressRange;

pub trait MemorySegment {
    fn get_range(&self, machine: &Machine) -> AddressRange;

    const NAME: &'static str;
}

pub trait StaticMemorySegment {
    fn get_static_range(machine: &Machine) -> AddressRange;

    const NAME: &'static str;
}

impl<T> MemorySegment for T where T: StaticMemorySegment {
    fn get_range(&self, machine: &Machine) -> AddressRange {
        T::get_static_range(machine)
    }

    const NAME: &'static str = T::NAME;
}

pub struct CallStackSegment {}

impl StaticMemorySegment for CallStackSegment {
    fn get_static_range(machine: &Machine) -> AddressRange {
        machine.stacks_border..=*machine.memory.address_range().end()
    }

    const NAME: &'static str = "Call stack";
}

pub struct DataStackSegment {}

impl StaticMemorySegment for DataStackSegment {
    fn get_static_range(machine: &Machine) -> AddressRange {
        machine.dict_ptr..=machine.stacks_border.wrapping_sub(1)
    }

    const NAME: &'static str = "Data stack";
}

pub struct UsedDictionarySegment {}

impl StaticMemorySegment for UsedDictionarySegment {
    fn get_static_range(machine: &Machine) -> AddressRange {
        *machine.memory.address_range().start()..=machine.dict_ptr - 1
    }

    const NAME: &'static str = "Used dictionary space";
}

pub struct FreeDataSegment {}

impl StaticMemorySegment for FreeDataSegment {
    fn get_static_range(machine: &Machine) -> AddressRange {
        machine.dict_ptr..=machine.data_stack_ptr.wrapping_sub(1)
    }

    const NAME: &'static str = "Free data space";
}

pub struct DataSegment {}

impl StaticMemorySegment for DataSegment {
    fn get_static_range(machine: &Machine) -> AddressRange {
        *machine.memory.address_range().start()..=machine.data_stack_ptr.wrapping_sub(1)
    }

    const NAME: &'static str = "Data space";
}
