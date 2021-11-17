use crate::stream::sync::SyncStream;

/// Groups items in memory and stream them.
#[derive(new)]
pub struct SyncGroup<I: SyncStream<Item = R, Error = E>, R, S, T, E, KM, L, O>
where
    KM: Fn(R) -> (L, S),
    L: Eq,
    O: Fn(L, Vec<S>) -> T,
{
    input: I,
    key_map: KM,
    out: O,
    #[new(default)]
    current: Option<(L, Vec<S>)>,
}

impl<I: SyncStream<Item = R, Error = E>, R, S, T, E, KM, L, O> SyncStream
    for SyncGroup<I, R, S, T, E, KM, L, O>
where
    KM: Fn(R) -> (L, S),
    L: Eq,
    O: Fn(L, Vec<S>) -> T,
{
    type Item = T;
    type Error = E;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        while let Some(rec) = self.input.next()? {
            let (key, group_item) = (self.key_map)(rec);
            let ret = if let Some(current) = &mut self.current {
                if current.0 == key {
                    current.1.push(group_item);
                    None
                } else {
                    let complete = std::mem::replace(current, (key, vec![group_item]));
                    Some(Some(complete))
                }
            } else {
                self.current = Some((key, vec![group_item]));
                None
            };
            if let Some(ret) = ret {
                return Ok(ret.map(|(key, group)| (self.out)(key, group)));
            }
        }
        let s = std::mem::take(&mut self.current);
        Ok(s.map(|(key, group)| (self.out)(key, group)))
    }
}

#[test]
fn should_group_stream() {
    use crate::vector::VectorStream;

    let mut stream = SyncGroup::new(
        VectorStream::new(vec!["", "a", "A", "z", "ZZZ"]),
        |word| {
            (
                word.chars().next().map(|c| c.to_lowercase().to_string()),
                word,
            )
        },
        |first_char, group| (first_char, group),
    );
    assert_matches!(stream.next(), Ok(Some((None, w))) if w == vec![""]);
    assert_matches!(stream.next(), Ok(Some((Some(fc), w))) if fc == "a" && w == vec!["a", "A"]);
    assert_matches!(stream.next(), Ok(Some((Some(fc), w))) if fc == "z" && w == vec!["z", "ZZZ"]);
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
