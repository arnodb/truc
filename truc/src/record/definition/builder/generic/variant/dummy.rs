use crate::record::definition::{DatumDefinitionCollection, DatumId};

/// Record variant builder that simply removes data and appends data at the end of the record.
pub fn append_data<D>(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    _datum_definitions: &mut DatumDefinitionCollection<D>,
) -> Vec<DatumId> {
    data.retain(|datum_id| !data_to_remove.iter().any(|did| did == datum_id));

    for &datum_id in &data_to_add {
        data.push(datum_id);
    }

    data
}

/// Record variant builder that simply removes data and appends data in "reverse" order at the end
/// of the record.
///
/// Useful to simulate fuzzing.
pub fn append_data_reverse<D>(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    _datum_definitions: &mut DatumDefinitionCollection<D>,
) -> Vec<DatumId> {
    data.retain(|datum_id| !data_to_remove.iter().any(|did| did == datum_id));

    for &datum_id in data_to_add.iter().rev() {
        data.push(datum_id);
    }

    data
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{append_data, append_data_reverse};
    use crate::record::definition::{DatumDefinitionCollection, DatumId};

    fn add(name: &str, datum_definitions: &mut DatumDefinitionCollection<()>) -> DatumId {
        datum_definitions.push(name.to_owned(), ())
    }

    fn data_to_text<'a>(
        data: &[DatumId],
        datum_definitions: &'a DatumDefinitionCollection<()>,
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
            add("f1", &mut datum_definitions),
            add("f2", &mut datum_definitions),
            add("f3", &mut datum_definitions),
            add("f4", &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add("g1", &mut datum_definitions);
        let new_id2 = add("g2", &mut datum_definitions);

        let actual_data = append_data(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "f2", "f3", "f4", "g1", "g2"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                datum.name()
            },
            "g1"
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                datum.name()
            },
            "g2"
        );
    }

    #[test]
    fn should_append_data_reverse() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // xxxx ____ ____ xxxx ____ xxxx ____ xxxx 2222 2222 1111
        let data = [
            add("f1", &mut datum_definitions),
            add("f2", &mut datum_definitions),
            add("f3", &mut datum_definitions),
            add("f4", &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add("g1", &mut datum_definitions);
        let new_id2 = add("g2", &mut datum_definitions);

        let actual_data =
            append_data_reverse(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "f2", "f3", "f4", "g2", "g1"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                datum.name()
            },
            "g1"
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                datum.name()
            },
            "g2"
        );
    }
}
