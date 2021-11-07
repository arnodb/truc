#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate derive_new;

pub mod record;

#[cfg(test)]
mod tests {
    use crate::record::definition::RecordDefinitionBuilder;

    #[test]
    fn it_works() {
        let mut definition = RecordDefinitionBuilder::new();

        let a = definition.add_datum::<u32>();
        let b = definition.add_datum::<u32>();
        let rv1 = definition.close_record_variant();

        let c = definition.add_datum::<u32>();
        let rv2 = definition.close_record_variant();

        definition.remove_datum(a);
        let rv3 = definition.close_record_variant();

        let d = definition.add_datum::<u8>();
        let e = definition.add_datum::<u16>();
        let f = definition.add_datum::<u32>();
        let rv4 = definition.close_record_variant();

        let record = definition.build();
        let def = record.to_string();
        assert_eq!(
            def,
            format!(
                r#"{} [{} (4), {} (4)]
{} [{} (4), {} (4), {} (4)]
{} [void (4), {} (4), {} (4)]
{} [{} (1), {} (2), void (1), {} (4), {} (4), {} (4)]
"#,
                rv1, a, b, rv2, a, b, c, rv3, b, c, rv4, d, e, b, c, f
            )
        );
    }
}
