use crate::stream::sync::SyncStream;

/// Groups items in memory and stream them.
#[derive(new)]
pub struct Group<I: SyncStream<Item = R, Error = E>, R, S, E, G, C, H>
where
    G: Fn(R) -> S,
    C: Fn(&S, &R) -> bool,
    H: Fn(&mut S, R),
{
    input: I,
    group: G,
    eq: C,
    extend_group: H,
    #[new(default)]
    buffer: Option<S>,
    #[new(default)]
    end_of_input: bool,
}

impl<I: SyncStream<Item = R, Error = E>, R, S, E, G, C, H> SyncStream for Group<I, R, S, E, G, C, H>
where
    G: Fn(R) -> S,
    C: Fn(&S, &R) -> bool,
    H: Fn(&mut S, R),
{
    type Item = S;
    type Error = E;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        if !self.end_of_input {
            while let Some(rec) = self.input.next()? {
                if let Some(buffer) = &mut self.buffer {
                    if !(self.eq)(buffer, &rec) {
                        let ret = self.buffer.replace((self.group)(rec));
                        return Ok(ret);
                    } else {
                        (self.extend_group)(buffer, rec);
                    }
                } else {
                    self.buffer = Some((self.group)(rec));
                }
            }
            self.end_of_input = true;
        }
        Ok(self.buffer.take())
    }
}

#[test]
fn should_group_stream() {
    use crate::vector::VectorStream;

    let mut stream = Group::new(
        VectorStream::new(vec!["", "a", "A", "z", "ZZZ", "zero", "Zorro"]),
        |word| word,
        |group, word| {
            let group_fc = group.chars().next().map(|c| c.to_lowercase());
            let word_fc = word.chars().next().map(|c| c.to_lowercase());
            match (group_fc, word_fc) {
                (Some(a), Some(b)) => a.eq(b),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
        },
        |_group, _word| {},
    );
    assert_matches!(stream.next(), Ok(Some(v)) if v == "");
    assert_matches!(stream.next(), Ok(Some(v)) if v == "a");
    assert_matches!(stream.next(), Ok(Some(v)) if v == "z");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}

#[test]
fn should_group_stream_and_count() {
    use crate::vector::VectorStream;

    let mut stream = Group::new(
        VectorStream::new(vec!["", "a", "A", "ZZZ", "z", "zero", "Zorro"]),
        |word| (word, 1),
        |group, word| {
            let group_fc = group.0.chars().next().map(|c| c.to_lowercase());
            let word_fc = word.chars().next().map(|c| c.to_lowercase());
            match (group_fc, word_fc) {
                (Some(a), Some(b)) => a.eq(b),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
        },
        |group, _word| group.1 += 1,
    );
    assert_matches!(stream.next(), Ok(Some((v, 1))) if v == "");
    assert_matches!(stream.next(), Ok(Some((v, 2))) if v == "a");
    assert_matches!(stream.next(), Ok(Some((v, 4))) if v == "ZZZ");
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}

#[test]
fn should_group_stream_and_aggregate_into_vactor() {
    use crate::vector::VectorStream;

    let mut stream = Group::new(
        VectorStream::new(vec![("a", 12), ("a", 12), ("a", 42), ("b", 42)]),
        |input| (input.0, vec![input.1]),
        |a: &(&str, Vec<i32>), b| (&a.0).eq(&b.0),
        |group, input| group.1.push(input.1),
    );
    assert_matches!(stream.next(), Ok(Some((v, g))) if v == "a" && g == vec![12, 12, 42]);
    assert_matches!(stream.next(), Ok(Some((v, g))) if v == "b" && g == vec![42]);
    // End of stream
    assert_matches!(stream.next(), Ok(None));
    assert_matches!(stream.next(), Ok(None));
}
