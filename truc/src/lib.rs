#![cfg_attr(test, allow(clippy::many_single_char_names))]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate quote;

pub mod generator;
pub mod record;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use crate::record::{definition::RecordDefinitionBuilder, type_resolver::HostTypeResolver};

    #[test]
    fn it_works() {
        let type_resolver = HostTypeResolver;

        let mut definition = RecordDefinitionBuilder::new(&type_resolver);

        let a = definition.add_datum::<u32, _>("a");
        let b = definition.add_datum::<u32, _>("b");
        let rv1 = definition.close_record_variant();

        let c = definition.add_datum::<u32, _>("c");
        let rv2 = definition.close_record_variant();

        definition.remove_datum(a);
        let rv3 = definition.close_record_variant();

        let d = definition.add_datum::<u8, _>("d");
        let e = definition.add_datum::<u16, _>("e");
        let f = definition.add_datum::<u32, _>("f");
        let rv4 = definition.close_record_variant();

        let definition = definition.build();
        let def = definition.to_string();
        assert_eq!(
            def,
            format!(
                concat!(
                    // rv1
                    "{} [",
                    "{}: a (u32, align 4, offset 0, size 4), ",
                    "{}: b (u32, align 4, offset 4, size 4)",
                    "]\n",
                    // rv2
                    "{} [",
                    "{}: a (u32, align 4, offset 0, size 4), ",
                    "{}: b (u32, align 4, offset 4, size 4), ",
                    "{}: c (u32, align 4, offset 8, size 4)",
                    "]\n",
                    // rv3
                    "{} [",
                    "(void, 4), ",
                    "{}: b (u32, align 4, offset 4, size 4), ",
                    "{}: c (u32, align 4, offset 8, size 4)",
                    "]\n",
                    // rv4
                    "{} [",
                    "{}: d (u8, align 1, offset 0, size 1), ",
                    "(void, 3), ",
                    "{}: b (u32, align 4, offset 4, size 4), ",
                    "{}: c (u32, align 4, offset 8, size 4), ",
                    "{}: e (u16, align 2, offset 12, size 2), ",
                    "(void, 2), ",
                    "{}: f (u32, align 4, offset 16, size 4)",
                    "]\n"
                ),
                rv1, a, b, rv2, a, b, c, rv3, b, c, rv4, d, b, c, e, f,
            )
        );
    }
}
