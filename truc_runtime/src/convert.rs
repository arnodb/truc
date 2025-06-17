use std::{
    any::type_name,
    mem::{ManuallyDrop, MaybeUninit},
};

/// The result of a record conversion in a call to [convert_vec_in_place].
pub enum VecElementConversionResult<T> {
    /// The record has been converted to the attached value.
    Converted(T),
    /// The record has been abandonned.
    Abandonned,
}

/// Converts a vector of `T` to a vector of `U` where `T` and `U` have the same size in memory and
/// the same alignment rule according to the Rust compiler.
///
/// If the provided converter panics then your memory is safe: no invalid access is performed,
/// values that need to be dropped are dropped.
///
/// Note: the 2 required conditions are checked at runtime. However it is reasonably expected that
/// those runtime checks are optimized statically by the compiler: NOOP or pure panic.
pub fn convert_vec_in_place<T, U, C>(input: Vec<T>, convert: C) -> Vec<U>
where
    C: Fn(T, Option<&mut U>) -> VecElementConversionResult<U> + std::panic::RefUnwindSafe,
{
    try_convert_vec_in_place(input, |t, u| -> Result<_, ()> { Ok(convert(t, u)) }).unwrap()
}

/// Converts a vector of `T` to a vector of `U` where `T` and `U` have the same size in memory and
/// the same alignment rule according to the Rust compiler.
///
/// If the provided converter panics then your memory is safe: no invalid access is performed,
/// values that need to be dropped are dropped.
///
/// Note: the 2 required conditions are checked at runtime. However it is reasonably expected that
/// those runtime checks are optimized statically by the compiler: NOOP or pure panic.
pub fn try_convert_vec_in_place<T, U, C, E>(input: Vec<T>, convert: C) -> Result<Vec<U>, E>
where
    C: Fn(T, Option<&mut U>) -> Result<VecElementConversionResult<U>, E>
        + std::panic::RefUnwindSafe,
{
    // It would be nice to assert that statically. We could use a trait that indicates the
    // invariant but this would have two drawbacks:
    //
    // - you have to trust the implementations of the trait
    // - this would prevent from allowing conversions from any type T to any other type U where
    // they both have the same memory layout
    //
    // Side note: those runtime assertions are optimised statically: either code without
    // assertion code (the happy path), or pure panic (the incorrect path).
    assert_eq!(
        std::mem::size_of::<T>(),
        std::mem::size_of::<U>(),
        "size_of {} vs {}",
        type_name::<T>(),
        type_name::<U>()
    );
    assert_eq!(
        std::mem::align_of::<T>(),
        std::mem::align_of::<U>(),
        "align_of {} vs {}",
        type_name::<T>(),
        type_name::<U>()
    );

    // Let's take control, we know what we're doing
    let mut manually_drop = ManuallyDrop::new(input);
    let slice = manually_drop.as_mut_slice();

    // From now on, slice is divided into 3 areas:
    //
    // - 0..first_moved: elements of type U (to be dropped by the panic handler)
    // - first_moved..first_ttt: dropped elements
    // - first_ttt..: elements of type T (to be dropped by the panic handler)
    //
    // This must remain true until the end so that the panic handler drops elements correctly.
    let mut first_moved = 0;
    let mut first_ttt = 0;

    let maybe_panic =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), E> {
            while first_ttt < slice.len() {
                // Bring one T back into auto-drop land
                let ttt = {
                    let mut ttt = MaybeUninit::<T>::uninit();
                    unsafe {
                        std::ptr::copy_nonoverlapping(&slice[first_ttt], ttt.as_mut_ptr(), 1);
                    }
                    // The element in the slice is now moved
                    first_ttt += 1;
                    unsafe { ttt.assume_init() }
                };

                // Convert it
                let converted = convert(
                    ttt,
                    // Pass a mutable reference on the preceeding converted element if it exists
                    if first_moved > 0 {
                        Some(unsafe { &mut *(&mut slice[first_moved - 1] as *mut T).cast() })
                    } else {
                        None
                    },
                )?;

                // Store the result
                match converted {
                    VecElementConversionResult::Converted(uuu) => {
                        unsafe {
                            std::ptr::write((&mut slice[first_moved] as *mut T).cast(), uuu);
                        }
                        // The element is now converted
                        first_moved += 1;
                    }
                    VecElementConversionResult::Abandonned => {
                        // The element has been abandonned by the converter
                    }
                }
            }
            Ok(())
        }));

    let clean_on_error = || {
        // Bring Us back into auto-drop land
        for element in &slice[0..first_moved] {
            let mut uuu = MaybeUninit::<U>::uninit();
            unsafe {
                std::ptr::copy_nonoverlapping(&*(element as *const T).cast(), uuu.as_mut_ptr(), 1);
                uuu.assume_init();
            }
        }
        // Bring Ts back into auto-drop land
        for element in &slice[first_ttt..slice.len()] {
            let mut ttt = MaybeUninit::<T>::uninit();
            unsafe {
                std::ptr::copy_nonoverlapping(element, ttt.as_mut_ptr(), 1);
                ttt.assume_init();
            }
        }
    };

    match maybe_panic {
        Ok(Ok(())) => {
            unsafe {
                manually_drop.set_len(first_moved);
            }
            Ok(unsafe {
                std::mem::transmute::<Vec<T>, Vec<U>>(ManuallyDrop::into_inner(manually_drop))
            })
        }
        Ok(Err(err)) => {
            clean_on_error();
            Err(err)
        }
        Err(err) => {
            clean_on_error();
            panic!("{:?}", err);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use super::*;

    struct CountDrop1 {
        value: usize,
        dropped: Arc<AtomicUsize>,
    }

    impl Drop for CountDrop1 {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct CountDrop1000 {
        #[allow(unused)]
        value: usize,
        dropped: Arc<AtomicUsize>,
    }

    impl Drop for CountDrop1000 {
        fn drop(&mut self) {
            self.dropped.fetch_add(1000, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_drop_all_input_and_reduced_output() {
        let dropped1 = Arc::new(AtomicUsize::new(0));
        let dropped2 = Arc::new(AtomicUsize::new(0));

        let mut input = Vec::new();
        for value in 0..32 {
            input.push(CountDrop1 {
                value,
                dropped: dropped1.clone(),
            });
        }

        let output = convert_vec_in_place::<CountDrop1, CountDrop1000, _>(input, |rec, _| {
            if rec.value % 4 == 0 {
                VecElementConversionResult::Converted(CountDrop1000 {
                    value: rec.value,
                    dropped: dropped2.clone(),
                })
            } else {
                VecElementConversionResult::Abandonned
            }
        });

        assert_eq!(output.len(), 8);

        // All 1s are dropped
        assert_eq!(dropped1.load(Ordering::Relaxed), 32);

        drop(output);

        // All 8 converted 2s are dropped
        assert_eq!(dropped2.load(Ordering::Relaxed), 8000);
    }

    #[test]
    fn test_drops_on_panic() {
        let dropped1 = Arc::new(AtomicUsize::new(0));
        let dropped2 = Arc::new(AtomicUsize::new(0));

        let mut input = Vec::new();
        for value in 0..32 {
            input.push(CountDrop1 {
                value,
                dropped: dropped1.clone(),
            });
        }

        let panic = std::panic::catch_unwind(|| {
            convert_vec_in_place::<CountDrop1, CountDrop1000, _>(input, |rec, _| {
                if rec.value == 23 {
                    panic!("boom");
                } else if rec.value % 4 == 0 {
                    VecElementConversionResult::Converted(CountDrop1000 {
                        value: rec.value,
                        dropped: dropped2.clone(),
                    })
                } else {
                    VecElementConversionResult::Abandonned
                }
            })
        });
        assert!(panic.is_err());

        // All 1s are dropped
        assert_eq!(dropped1.load(Ordering::Relaxed), 32);

        // All 6 (only) converted 2s are dropped
        assert_eq!(dropped2.load(Ordering::Relaxed), 6000);
    }
}
