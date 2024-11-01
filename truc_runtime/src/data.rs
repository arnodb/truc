use std::mem::MaybeUninit;

/// Internal data holder, heavily unsage, do not use it directly.
pub struct RecordMaybeUninit<const CAP: usize> {
    data: [MaybeUninit<u8>; CAP],
}

impl<const CAP: usize> RecordMaybeUninit<CAP> {
    /// Constructs an uninitialized record.
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
    fn test_record_getters() {
        const CAP: usize = std::mem::size_of::<u32>() * 2;

        let mut record = RecordMaybeUninit::<CAP>::default();
        for (i1, i2) in [
            (0x2cfed605, 0xa93696d0),
            (0xf62c11c5, 0xca28ccda),
            (0x6844c3a7, 0x1979719d),
            (0xfad56b4e, 0x43f160da),
            (0xf287ec76, 0x30850690),
            (0x00be3837, 0x55b6dd5b),
            (0xa2a359c2, 0x11a32b39),
            (0xd555b28d, 0xeb2dde75),
        ] {
            unsafe {
                *record.get_mut::<u32>(0) = i1;
                *record.get_mut::<u32>(4) = i2;
                assert_eq!(*record.get::<u32>(0), i1);
                assert_eq!(*record.get::<u32>(4), i2);
                *record.get_mut::<u32>(0) = i2;
                *record.get_mut::<u32>(4) = i1;
                assert_eq!(*record.get::<u32>(0), i2);
                assert_eq!(*record.get::<u32>(4), i1);
            }
        }
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
