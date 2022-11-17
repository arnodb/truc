#![cfg_attr(test, allow(clippy::many_single_char_names))]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate quote;

pub mod generator;
pub mod record;

#[cfg(test)]
mod tests {
    use crate::record::definition::RecordDefinitionBuilder;

    #[test]
    fn it_works() {
        let mut definition = RecordDefinitionBuilder::new();

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
                r#"{} [{} (u32, 4), {} (u32, 4)]
{} [{} (u32, 4), {} (u32, 4), {} (u32, 4)]
{} [(void, 4), {} (u32, 4), {} (u32, 4)]
{} [{} (u8, 1), {} (u16, 2), (void, 1), {} (u32, 4), {} (u32, 4), {} (u32, 4)]
"#,
                rv1, a, b, rv2, a, b, c, rv3, b, c, rv4, d, e, b, c, f
            )
        );
    }
}
