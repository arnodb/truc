use std::{
    any::type_name,
    mem::{ManuallyDrop, MaybeUninit},
};

pub enum VecElementConversionResult<T> {
    Converted(T),
    Abandonned,
}

pub fn convert_vec_in_place<T, U, C>(input: Vec<T>, convert: C) -> Vec<U>
where
    C: Fn(T, Option<&mut U>) -> VecElementConversionResult<U> + std::panic::RefUnwindSafe,
{
    // It would be nice to assert that statically. We could use a trait that indicates the
    // invariant but this would have two drawbacks:
    //
    // - you have to trust the implementations of the trait
    // - this would prevent from allowing
    // conversions from any type T to any other type U where they both have the same memory
    // layout
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

    let maybe_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
            );

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
    }));

    match maybe_panic {
        Ok(()) => {
            unsafe {
                manually_drop.set_len(first_moved);
            }
            unsafe { std::mem::transmute(ManuallyDrop::into_inner(manually_drop)) }
        }
        Err(err) => {
            // Bring Us back into auto-drop land
            for element in &slice[0..first_moved] {
                let mut uuu = MaybeUninit::<U>::uninit();
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        &*(element as *const T).cast(),
                        uuu.as_mut_ptr(),
                        1,
                    );
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
            panic!("{:?}", err);
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_drop_all_input_and_reduced_output() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        };

        struct CountDrop {
            value: usize,
            dropped: Arc<AtomicUsize>,
        }

        impl Drop for CountDrop {
            fn drop(&mut self) {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }

        let dropped1 = Arc::new(AtomicUsize::new(0));
        let dropped2 = Arc::new(AtomicUsize::new(0));

        let mut input = Vec::new();
        for value in 0..32 {
            input.push(CountDrop {
                value,
                dropped: dropped1.clone(),
            });
        }

        let output = convert_vec_in_place::<CountDrop, CountDrop, _>(input, |rec, _| {
            if rec.value % 4 == 0 {
                VecElementConversionResult::Converted(CountDrop {
                    value: rec.value,
                    dropped: dropped2.clone(),
                })
            } else {
                VecElementConversionResult::Abandonned
            }
        });

        assert_eq!(output.len(), 8);

        assert_eq!(dropped1.load(Ordering::Relaxed), 32);

        drop(output);

        assert_eq!(dropped2.load(Ordering::Relaxed), 8);
    }
}
