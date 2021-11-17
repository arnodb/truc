/// A fallible stream with all the functional instrumentation to chain streams gracefully.
pub trait SyncStream {
    type Item;
    type Error;

    /// Gets the next item of this stream if any.
    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error>;

    /// Changes the error and/or error type without changing the items.
    fn map_err<O, F>(self, op: O) -> MapErr<Self, O>
    where
        Self: Sized,
        O: FnMut(Self::Error) -> F,
    {
        MapErr::new(self, op)
    }

    /// Maps items until an error is encountered.
    fn and_then_map<B, F>(self, f: F) -> AndThenMap<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Result<B, Self::Error>,
    {
        AndThenMap::new(self, f)
    }

    /// Transposes a `Result<Option>` into an `Option<Result>`, mainly in order to use the
    /// [`Iterator`] machinery.
    fn transpose(self) -> SyncStreamIter<Self>
    where
        Self: Sized,
    {
        SyncStreamIter(self)
    }
}

/// See [`SyncStream::map_err`].
pub struct MapErr<T, O> {
    stream: T,
    op: O,
}

impl<T, O> MapErr<T, O> {
    fn new(stream: T, op: O) -> Self {
        Self { stream, op }
    }
}

impl<T, O, F> SyncStream for MapErr<T, O>
where
    T: SyncStream,
    O: FnMut(T::Error) -> F,
{
    type Item = T::Item;
    type Error = F;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.stream.next().map_err(&mut self.op)
    }
}

/// See [`SyncStream::and_then_map`].
pub struct AndThenMap<T, F> {
    stream: T,
    f: F,
}

impl<T, F> AndThenMap<T, F> {
    fn new(stream: T, f: F) -> Self {
        Self { stream, f }
    }
}

impl<T, B, F> SyncStream for AndThenMap<T, F>
where
    T: SyncStream,
    F: FnMut(T::Item) -> Result<B, T::Error>,
{
    type Item = B;
    type Error = T::Error;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.stream
            .next()
            .and_then(|item| item.map(&mut self.f).transpose())
    }
}

impl<T: SyncStream + ?Sized> SyncStream for Box<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        (**self).next()
    }
}

/// See [`SyncStream::transpose`].
pub struct SyncStreamIter<T: SyncStream>(T);

impl<T: SyncStream> Iterator for SyncStreamIter<T> {
    type Item = Result<T::Item, T::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().transpose()
    }
}
