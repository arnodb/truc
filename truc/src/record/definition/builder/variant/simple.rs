use std::collections::BTreeMap;

use super::{align_bytes, DataUpdater};
use crate::record::definition::{DatumDefinition, DatumDefinitionCollection, DatumId};

#[derive(Debug)]
struct Gap {
    start: usize,
    end: usize,
    datum_index: usize,
}

fn compute_initial_gaps(
    data: &[DatumId],
    datum_definitions: &DatumDefinitionCollection,
) -> Vec<Gap> {
    let mut last_offset = 0;
    data.iter()
        .enumerate()
        .filter_map(|(datum_index, &datum_id)| {
            let datum = datum_definitions
                .get(datum_id)
                .unwrap_or_else(|| panic!("datum #{}", datum_id));
            // Ignore empty data
            if datum.size() == 0 {
                return None;
            }
            if datum.offset > last_offset {
                let gap = Gap {
                    start: last_offset,
                    end: datum.offset,
                    datum_index,
                };
                last_offset = datum.offset + datum.size();
                Some(gap)
            } else {
                last_offset = datum.offset + datum.size();
                None
            }
        })
        .collect()
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
struct FullGap(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FittedDatumKind {
    StartOfGap,
    EndOfGap,
}

#[derive(Debug)]
struct FittedDatum {
    kind: FittedDatumKind,
    gap_index: usize,
    gap_before: usize,
    gap_after: usize,
    datum_start: usize,
    datum_end: usize,
}

impl FittedDatum {
    fn selection_value(&self) -> usize {
        match self.kind {
            FittedDatumKind::StartOfGap => self.datum_end,
            FittedDatumKind::EndOfGap => self.datum_start,
        }
    }
}

fn fit_datum_to_gap(
    gap_index: usize,
    gap: &Gap,
    datum: &DatumDefinition,
) -> Option<(FullGap, FittedDatum)> {
    let datum_start = align_bytes(gap.start, datum.type_align());
    let datum_end = datum_start + datum.size();
    if gap.end >= datum_end {
        let gap_before = datum_start - gap.start;
        let gap_after = gap.end - datum_end;
        Some((
            FullGap(gap_before + gap_after),
            FittedDatum {
                kind: FittedDatumKind::StartOfGap,
                gap_index,
                gap_before,
                gap_after,
                datum_start,
                datum_end,
            },
        ))
    } else {
        None
    }
}

// The one that best realigns to the higest power of 2 wins
fn select_best(
    mut first: usize,
    first_result: impl FnOnce() -> FittedDatum,
    mut second: usize,
    second_result: impl FnOnce() -> FittedDatum,
) -> FittedDatum {
    if first == 0 {
        // First, you win
        first_result()
    } else if second == 0 {
        // Second, you win
        second_result()
    } else {
        loop {
            // Second, you lose
            if second & 1 == 1 {
                break first_result();
            }
            // First, you lose
            if first & 1 == 1 {
                break second_result();
            }
            first >>= 1;
            second >>= 1;
        }
    }
}

fn select_start_or_end_of_gap(start_of_gap: FittedDatum, type_align: usize) -> FittedDatum {
    assert_eq!(start_of_gap.kind, FittedDatumKind::StartOfGap);
    let &FittedDatum {
        kind: _,
        gap_index,
        gap_before,
        gap_after,
        datum_start,
        datum_end,
    } = &start_of_gap;
    let delta = gap_after / type_align * type_align;
    if delta > 0 {
        select_best(
            datum_end,
            || start_of_gap,
            datum_start + delta,
            || FittedDatum {
                kind: FittedDatumKind::EndOfGap,
                gap_index,
                gap_before: gap_before + delta,
                gap_after: gap_after - delta,
                datum_start: datum_start + delta,
                datum_end: datum_end + delta,
            },
        )
    } else {
        start_of_gap
    }
}

/// Not so simple but yes, it tries to fill in the gaps in a simple way.
///
/// Exact gaps are prioritized, then gaps where the data can increase alignment boundaries.
pub fn simple(
    mut data: Vec<DatumId>,
    data_to_add: Vec<DatumId>,
    data_to_remove: Vec<DatumId>,
    datum_definitions: &mut DatumDefinitionCollection,
) -> Vec<DatumId> {
    // Remove first to optimize space

    data.remove_data(data_to_remove.iter().cloned());

    // Then add

    let mut gaps = compute_initial_gaps(&data, datum_definitions);

    // By decreasing size order
    let data_to_add_by_size = {
        let mut map = BTreeMap::<usize, Vec<DatumId>>::new();
        for datum_id in data_to_add {
            let datum = datum_definitions
                .get(datum_id)
                .unwrap_or_else(|| panic!("datum #{}", datum_id));
            map.entry(datum.size()).or_default().push(datum_id);
        }
        map
    };

    for datum_id in data_to_add_by_size.into_values().rev().flatten() {
        let datum = datum_definitions
            .get(datum_id)
            .unwrap_or_else(|| panic!("datum #{}", datum_id));

        let mut fitted_by_full_gap = BTreeMap::<FullGap, Vec<FittedDatum>>::new();

        for (gap_index, gap) in gaps.iter().enumerate() {
            // No room
            if gap.end - gap.start < datum.size() {
                continue;
            }

            if let Some((full_gap, fitted)) = fit_datum_to_gap(gap_index, gap, datum) {
                let exact = fitted.gap_before == 0 && fitted.gap_after == 0;

                fitted_by_full_gap.entry(full_gap).or_default().push(fitted);

                if exact {
                    // We can stop now, we got an early winner
                    break;
                }
            }

            // Fallback
        }

        struct InsertData {
            gap_index: usize,
            datum_start: usize,
            replace_gap_with: Option<(Gap, Option<Gap>)>,
        }

        let insert: Option<InsertData> = if !fitted_by_full_gap.is_empty() {
            // Found an almost exact gap
            let fitted = fitted_by_full_gap.into_values().next().unwrap();

            let chosen = {
                let mut iter = fitted.into_iter();
                let first = select_start_or_end_of_gap(iter.next().unwrap(), datum.type_align());
                iter.fold(first, |prev, current| {
                    let selected_current = select_start_or_end_of_gap(current, datum.type_align());
                    select_best(
                        prev.selection_value(),
                        || prev,
                        selected_current.selection_value(),
                        || selected_current,
                    )
                })
            };

            let FittedDatum {
                kind: _,
                gap_index,
                gap_before,
                gap_after,
                datum_start,
                datum_end,
            } = chosen;

            let gap = &gaps[gap_index];

            let gap_before = (gap_before > 0).then(|| Gap {
                start: gap.start,
                end: gap.start + gap_before,
                datum_index: gap.datum_index,
            });

            let gap_after = (gap_after > 0).then(|| Gap {
                start: datum_end,
                end: gap.end,
                datum_index: gap.datum_index + 1,
            });

            Some(InsertData {
                gap_index,
                datum_start,
                replace_gap_with: match (gap_before, gap_after) {
                    (Some(b), ma) => Some((b, ma)),
                    (None, Some(a)) => Some((a, None)),
                    (None, None) => None,
                },
            })
        } else {
            None
        };

        // Now is the time to actually do things
        if let Some(InsertData {
            gap_index,
            datum_start,
            replace_gap_with,
        }) = insert
        {
            let gap = &gaps[gap_index];

            data.insert(gap.datum_index, datum_id);

            let datum_mut = datum_definitions
                .get_mut(datum_id)
                .unwrap_or_else(|| panic!("datum #{}", datum_id));
            datum_mut.offset = datum_start;

            let first_gap_after = match replace_gap_with {
                Some((replace_with_gap, Some(and_gap))) => {
                    gaps[gap_index] = replace_with_gap;
                    gaps.insert(gap_index + 1, and_gap);
                    gap_index + 2
                }
                Some((replace_with_gap, None)) => {
                    gaps[gap_index] = replace_with_gap;
                    gap_index + 1
                }
                None => {
                    gaps.remove(gap_index);
                    gap_index
                }
            };
            for gap in &mut gaps[first_gap_after..] {
                gap.datum_index += 1;
            }
        } else {
            let (gap_start, gap_end) = data.push_datum(datum_definitions, datum_id);
            if gap_end > gap_start {
                gaps.push(Gap {
                    start: gap_start,
                    end: gap_end,
                    datum_index: data.len() - 1,
                });
            }
        }
    }

    data
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::any::type_name;

    use super::simple;
    use crate::record::{
        definition::{builder::variant::align_bytes, DatumDefinitionCollection, DatumId},
        type_resolver::TypeInfo,
    };

    fn add<T: Copy>(
        name: &str,
        offset: usize,
        datum_definitions: &mut DatumDefinitionCollection,
    ) -> DatumId {
        if offset != usize::MAX {
            assert_eq!(align_bytes(offset, std::mem::align_of::<T>()), offset);
        }
        datum_definitions.push(
            name.to_owned(),
            offset,
            TypeInfo {
                name: type_name::<T>().to_owned(),
                size: std::mem::size_of::<T>(),
                align: std::mem::align_of::<T>(),
            },
            true,
        )
    }

    fn data_to_text<'a>(
        data: &[DatumId],
        datum_definitions: &'a DatumDefinitionCollection,
    ) -> Vec<&'a str> {
        data.iter()
            .map(|&d| datum_definitions.get(d).unwrap().name())
            .collect()
    }

    #[test]
    fn should_fill_exact_gap() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // 64 bits
        // xxxx ____ ____ xxxx 1111 xxxx 2222 2222 xxxx
        // 32 bits
        // xxxx 2222 2222 xxxx 1111 xxxx ____ ____ xxxx
        let data = [
            add::<u32>("f1", 0, &mut datum_definitions),
            add::<u32>("f2", 12, &mut datum_definitions),
            add::<u32>("f3", 20, &mut datum_definitions),
            add::<u32>("f4", 32, &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add::<u32>("g1", usize::MAX, &mut datum_definitions);
        let new_id2 = add::<u64>("g2", usize::MAX, &mut datum_definitions);

        let actual_data = simple(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        #[cfg(target_pointer_width = "64")]
        let expected_data_text = vec!["f1", "f2", "g1", "f3", "g2", "f4"];
        #[cfg(not(target_pointer_width = "64"))]
        let expected_data_text = vec!["f1", "g2", "f2", "g1", "f3", "f4"];
        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            expected_data_text
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                (datum.name(), datum.offset)
            },
            ("g1", 16)
        );

        #[cfg(target_pointer_width = "64")]
        let expected = ("g2", 24);
        #[cfg(not(target_pointer_width = "64"))]
        let expected = ("g2", 4);
        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                (datum.name(), datum.offset)
            },
            expected
        );
    }

    #[test]
    fn should_fill_ambiguous_gap_1() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // xxxx 1111 ____ xx__ ____ __xx
        // It better realigns
        //      ^^^^ here
        //                than ^^^^ here
        let data = [
            add::<u32>("f1", 0, &mut datum_definitions),
            add::<u16>("f2", 12, &mut datum_definitions),
            add::<u16>("f3", 22, &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add::<u32>("g1", usize::MAX, &mut datum_definitions);

        let actual_data = simple(data, vec![new_id1], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "g1", "f2", "f3"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                (datum.name(), datum.offset)
            },
            ("g1", 4)
        );
    }

    #[test]
    fn should_fill_ambiguous_gap_2() {
        let mut datum_definitions = DatumDefinitionCollection::default();
        // xxxx 2222 ____ ____ 1111 xxxx
        let data = [
            add::<u32>("f1", 0, &mut datum_definitions),
            add::<u32>("f2", 20, &mut datum_definitions),
        ]
        .to_vec();

        let new_id1 = add::<u32>("g1", usize::MAX, &mut datum_definitions);
        let new_id2 = add::<u32>("g2", usize::MAX, &mut datum_definitions);

        let actual_data = simple(data, vec![new_id1, new_id2], vec![], &mut datum_definitions);

        assert_eq!(
            data_to_text(&actual_data, &datum_definitions),
            vec!["f1", "g2", "g1", "f2"]
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id1).unwrap();
                (datum.name(), datum.offset)
            },
            ("g1", 16)
        );

        assert_eq!(
            {
                let datum = datum_definitions.get(new_id2).unwrap();
                (datum.name(), datum.offset)
            },
            ("g2", 4)
        );
    }
}
