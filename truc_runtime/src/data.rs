use std::mem::MaybeUninit;

pub struct RecordMaybeUninit<const CAP: usize> {
    data: [MaybeUninit<u8>; CAP],
}

impl<const CAP: usize> RecordMaybeUninit<CAP> {
    pub fn new() -> Self {
        Self {
            data: unsafe { std::mem::MaybeUninit::uninit().assume_init() },
        }
    }

    /// Reads an object of type `T` back from the record at offset `offset`.
    ///
    /// # Safety
    ///
    /// This function should not be called by anything but truc-generated code. It is used to put
    /// data written by [`Self::write`] back in a droppable state.
    pub unsafe fn read<T>(&self, offset: usize) -> T {
        std::ptr::read((self.data.as_ptr().add(offset) as *const u8).cast())
    }

    /// Stores an object of type `T` in the record at offset `offset`.
    ///
    /// # Safety
    ///
    /// This function should not be called by anything but truc-generated code which is also
    /// responsible for dropping the data by reading the object (see [`Self::read`]).
    pub unsafe fn write<T>(&mut self, offset: usize, t: T) {
        std::ptr::write((self.data.as_ptr().add(offset) as *mut u8).cast(), t);
    }

    /// Gets a reference to object of type `T` from the record at offset `offset`.
    ///
    /// # Safety
    ///
    /// This function should not be called by anything but truc-generated code.
    pub unsafe fn get<T>(&self, offset: usize) -> &T {
        &*(self.data.as_ptr().add(offset) as *mut u8).cast()
    }

    /// Gets a mutable reference to object of type `T` from the record at offset `offset`.
    ///
    /// # Safety
    ///
    /// This function should not be called by anything but truc-generated code.
    pub unsafe fn get_mut<T>(&mut self, offset: usize) -> &mut T {
        &mut *(self.data.as_ptr().add(offset) as *mut u8).cast()
    }
}

impl<const CAP: usize> Default for RecordMaybeUninit<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_size_of_record() {
        assert_eq!(std::mem::size_of::<RecordMaybeUninit<0>>(), 0);
        assert_eq!(std::mem::size_of::<RecordMaybeUninit<1>>(), 1);
        assert_eq!(std::mem::size_of::<RecordMaybeUninit<12>>(), 12);
        assert_eq!(std::mem::size_of::<RecordMaybeUninit<42>>(), 42);
    }

    #[test]
    fn test_record_write_read_drop() {
        static mut COUNTER: usize = 0;

        struct Foo {}

        impl Drop for Foo {
            fn drop(&mut self) {
                unsafe {
                    COUNTER += 1;
                }
            }
        }

        const CAP: usize = std::mem::size_of::<Foo>();

        let mut record = RecordMaybeUninit::<CAP>::new();
        unsafe {
            record.write(0, Foo {});
        }
        let foo = unsafe { record.read::<Foo>(0) };
        drop(foo);

        let counter = unsafe { COUNTER };
        assert_eq!(counter, 1);
    }
}
