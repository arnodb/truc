use super::NativeDataUpdater;
use crate::record::definition::{DatumDefinitionCollection, DatumId, NativeDatumDetails};

pub fn append_data(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
) -> Vec<DatumId> {
    data.remove_data(data_to_remove.iter().cloned());

    for &datum_id in &data_to_add {
        data.push_datum(datum_definitions, datum_id);
    }

    data
}

pub fn append_data_reverse(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
) -> Vec<DatumId> {
    data.remove_data(data_to_remove.iter().cloned());

    for &datum_id in data_to_add.iter().rev() {
        data.push_datum(datum_definitions, datum_id);
    }

    data
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::any::type_name;

    use super::{append_data, append_data_reverse};
    use crate::record::{
        definition::{
            builder::native::variant::align_bytes, DatumDefinitionCollection, DatumId,
            NativeDatumDetails,
        },
        type_resolver::TypeInfo,
    };

    fn add<T: Copy>(
        name: &str,
        offset: usize,
        datum_definitions: &mut DatumDefinitionCollection<NativeDatumDetails>,
    ) -> DatumId {
        if offset != usize::MAX {
            assert_eq!(align_bytes(offset, std::mem::align_of::<T>()), offset);
        }
        datum_definitions.push(
            name.to_owned(),
            NativeDatumDetails::new(
                offset,
                TypeInfo {
                    name: type_name::<T>().to_owned(),
                    size: std::mem::size_of::<T>(),
                    align: std::mem::align_of::<T>(),
                },
                true,
            ),
        )
    }

    fn data_to_text<'a>(
        data: &[DatumId],
        datum_definitions: &'a DatumDefinitionCollection<NativeDatumDetails>,
    ) -> Vec<&'a str> {
        data.iter()
            .map(|&d| datum_definitions.get(d).unwrap().name())
            .collect()
    }

    #[test]
    fn should_append_data() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // 64 bits
        // xxxx ____ ____ xxxx ____ xxxx ____ xxxx 1111 ____ 2222 2222
        // 32 bits
        // xxxx ____ ____ xxxx ____ xxxx ____ xxxx 1111 2222 2222
        let data = [
            add::<u32>("f1", 0, &mut datum_definitions),
            add::<u32>("f2", 12, &mut datum_definitions),
            add::<u32>("f3", 20, &mut datum_definitions),
            add::<u32>("f4", 28, &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add::<u32>("g1", usize::MAX, &mut datum_definitions);
        let new_id2 = add::<u64>("g2", usize::MAX, &mut datum_definitions);

        let actual_data = append_data(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "f2", "f3", "f4", "g1", "g2"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                (datum.name(), datum.details().offset())
            },
            ("g1", 32)
        );

        #[cfg(target_pointer_width = "64")]
        let expected = ("g2", 40);
        #[cfg(not(target_pointer_width = "64"))]
        let expected = ("g2", 36);
        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                (datum.name(), datum.details().offset())
            },
            expected
        );
    }

    #[test]
    fn should_append_data_reverse() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // xxxx ____ ____ xxxx ____ xxxx ____ xxxx 2222 2222 1111
        let data = [
            add::<u32>("f1", 0, &mut datum_definitions),
            add::<u32>("f2", 12, &mut datum_definitions),
            add::<u32>("f3", 20, &mut datum_definitions),
            add::<u32>("f4", 28, &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add::<u32>("g1", usize::MAX, &mut datum_definitions);
        let new_id2 = add::<u64>("g2", usize::MAX, &mut datum_definitions);

        let actual_data =
            append_data_reverse(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "f2", "f3", "f4", "g2", "g1"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                (datum.name(), datum.details().offset())
            },
            ("g1", 40)
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                (datum.name(), datum.details().offset())
            },
            ("g2", 32)
        );
    }
}
