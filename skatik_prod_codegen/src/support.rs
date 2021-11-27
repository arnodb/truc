use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

#[derive(Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct FullyQualifiedName(Box<[Box<str>]>);

/// Fully qualified name helper
impl FullyQualifiedName {
    /// # Example
    ///
    /// ```
    /// use skatik_prod_codegen::support::FullyQualifiedName;
    ///
    /// let name = FullyQualifiedName::new("foo");
    /// assert_eq!(name.to_string(), "foo");
    /// ```
    pub fn new<S>(item: S) -> Self
    where
        S: ToString,
    {
        Self::new_n(Some(&item))
    }

    /// # Example
    ///
    /// ```
    /// use skatik_prod_codegen::support::FullyQualifiedName;
    ///
    /// let name = FullyQualifiedName::new_n(&["foo", "bar"]);
    /// assert_eq!(name.to_string(), "foo::bar");
    /// ```
    pub fn new_n<'s, I, S>(items: I) -> Self
    where
        I: IntoIterator<Item = &'s S>,
        S: ToString + 's,
    {
        Self(
            items
                .into_iter()
                .map(ToString::to_string)
                .inspect(|item| {
                    if item.contains(':') {
                        panic!(": forbidden in name fragment");
                    }
                })
                .map(String::into_boxed_str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    /// # Example
    ///
    /// ```
    /// use skatik_prod_codegen::support::FullyQualifiedName;
    ///
    /// let name = FullyQualifiedName::new("foo");
    /// let sub = name.sub("bar");
    /// assert_eq!(sub.to_string(), "foo::bar");
    /// ```
    pub fn sub<S>(&self, item: S) -> Self
    where
        S: ToString,
    {
        self.sub_n(Some(&item))
    }

    /// # Example
    ///
    /// ```
    /// use skatik_prod_codegen::support::FullyQualifiedName;
    ///
    /// let name = FullyQualifiedName::new("foo");
    /// let sub = name.sub_n(&["bar", "more"]);
    /// assert_eq!(sub.to_string(), "foo::bar::more");
    /// ```
    pub fn sub_n<'s, I, S>(&self, items: I) -> Self
    where
        I: IntoIterator<Item = &'s S>,
        S: ToString + 's,
    {
        Self(
            self.0
                .iter()
                .cloned()
                .chain(
                    items
                        .into_iter()
                        .map(ToString::to_string)
                        .inspect(|item| {
                            if item.contains(':') {
                                panic!(": forbidden in name fragment");
                            }
                        })
                        .map(String::into_boxed_str),
                )
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }
}

impl Display for FullyQualifiedName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, n) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }
            f.write_str(n)?;
        }
        Ok(())
    }
}

impl Deref for FullyQualifiedName {
    type Target = Box<[Box<str>]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
