//! Type resolution tools.

use std::collections::{btree_map::Entry, BTreeMap};

use serde::{Deserialize, Serialize};

use crate::record::type_name::{truc_dynamic_type_name, truc_type_name};

/// Type information (name, size and align) as given by the Rust compiler.
#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct TypeInfo {
    /// The type name as it can be used in Rust code.
    pub name: String,
    /// The type size given by `std::mem::size_of()`.
    pub size: usize,
    /// The type size given by `std::mem::align_of()`.
    pub align: usize,
}

/// Additional type information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicTypeInfo {
    /// Rust type information.
    pub info: TypeInfo,
    /// Indicates whether or not the type can be left safely uninitialized. Merely `Copy` types
    /// can be left uninitialized, any other type cannot.
    pub allow_uninit: bool,
}

/// Abstract type resolver trait.
pub trait TypeResolver {
    /// Gives the Rust type information for `T`.
    fn type_info<T>(&self) -> TypeInfo;

    /// Gives the dynamic type information `type_name`.
    fn dynamic_type_info(&self, type_name: &str) -> DynamicTypeInfo;
}

impl<R> TypeResolver for &R
where
    R: TypeResolver,
{
    fn type_info<T>(&self) -> TypeInfo {
        R::type_info::<T>(self)
    }

    fn dynamic_type_info(&self, type_name: &str) -> DynamicTypeInfo {
        R::dynamic_type_info(self, type_name)
    }
}

/// A type resolver that can give Rust type information only. Any call to
/// [dynamic_type_info](HostTypeResolver::dynamic_type_info) will panic.
pub struct HostTypeResolver;

impl TypeResolver for HostTypeResolver {
    /// Resolves the Rust type information by calling `std::mem::size_of()` and `std::mem::align_of()`.
    fn type_info<T>(&self) -> TypeInfo {
        TypeInfo {
            name: truc_type_name::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }

    /// Panics!
    fn dynamic_type_info(&self, _type_name: &str) -> DynamicTypeInfo {
        unimplemented!("HostTypeResolver cannot resolve dynamic types")
    }
}

macro_rules! add_type {
    ($resolver:ident, $type:ty) => {
        $resolver.add_type::<$type>();
        $resolver.add_type::<Option<$type>>();
    };
    ($resolver:ident, $type:ty, allow_uninit) => {
        $resolver.add_type_allow_uninit::<$type>();
        $resolver.add_type_allow_uninit::<Option<$type>>();
    };
}
macro_rules! add_type_and_arrays {
    ($resolver:ident, $type:ty) => {
        add_type!($resolver, $type);
        add_type!($resolver, [$type; 1]);
        add_type!($resolver, [$type; 2]);
        add_type!($resolver, [$type; 3]);
        add_type!($resolver, [$type; 4]);
        add_type!($resolver, [$type; 5]);
        add_type!($resolver, [$type; 6]);
        add_type!($resolver, [$type; 7]);
        add_type!($resolver, [$type; 8]);
        add_type!($resolver, [$type; 9]);
        add_type!($resolver, [$type; 10]);
    };
    ($resolver:ident, $type:ty, allow_uninit) => {
        add_type!($resolver, $type, allow_uninit);
        add_type!($resolver, [$type; 1], allow_uninit);
        add_type!($resolver, [$type; 2], allow_uninit);
        add_type!($resolver, [$type; 3], allow_uninit);
        add_type!($resolver, [$type; 4], allow_uninit);
        add_type!($resolver, [$type; 5], allow_uninit);
        add_type!($resolver, [$type; 6], allow_uninit);
        add_type!($resolver, [$type; 7], allow_uninit);
        add_type!($resolver, [$type; 8], allow_uninit);
        add_type!($resolver, [$type; 9], allow_uninit);
        add_type!($resolver, [$type; 10], allow_uninit);
    };
}

/// A type resolved that loads precomputed type information.
///
/// In addition to allowing a good level of customization, it is also very useful for
/// cross-compilation:
///
/// * compute data by running the resolution on the target platform
/// * serialize the data with `serde` to a file
/// * deserialize the file in the project to be cross-compiled
#[derive(Debug, From, Serialize, Deserialize)]
pub struct StaticTypeResolver {
    types: BTreeMap<String, DynamicTypeInfo>,
}

impl StaticTypeResolver {
    /// Creates an empty resolver.
    pub fn new() -> Self {
        Self {
            types: BTreeMap::new(),
        }
    }

    /// Adds a single type information to the data.
    pub fn add_type<T>(&mut self) {
        let type_name = truc_type_name::<T>();
        match self.types.entry(type_name.clone()) {
            Entry::Vacant(vacant) => {
                vacant.insert(DynamicTypeInfo {
                    info: TypeInfo {
                        name: type_name,
                        size: std::mem::size_of::<T>(),
                        align: std::mem::align_of::<T>(),
                    },
                    allow_uninit: false,
                });
            }
            Entry::Occupied(occupied) => {
                panic!(
                    "Type {} is already defined with {:?}",
                    type_name,
                    occupied.get()
                );
            }
        }
    }

    /// Adds a single `Copy` type information to the data.
    pub fn add_type_allow_uninit<T>(&mut self)
    where
        T: Copy,
    {
        let type_name = truc_type_name::<T>();
        match self.types.entry(type_name.clone()) {
            Entry::Vacant(vacant) => {
                vacant.insert(DynamicTypeInfo {
                    info: TypeInfo {
                        name: type_name,
                        size: std::mem::size_of::<T>(),
                        align: std::mem::align_of::<T>(),
                    },
                    allow_uninit: true,
                });
            }
            Entry::Occupied(occupied) => {
                panic!(
                    "Type {} is already defined with {:?}",
                    type_name,
                    occupied.get()
                );
            }
        }
    }

    /// Adds standard types to the data.
    ///
    /// It includes:
    ///
    /// * various types of integers and floating point numbers
    /// * `String`
    /// * `Box<str>`
    /// * `Vec<()>` which is enough to support any kind of vector
    pub fn add_std_types(&mut self) {
        add_type_and_arrays!(self, u8, allow_uninit);
        add_type_and_arrays!(self, u16, allow_uninit);
        add_type_and_arrays!(self, u32, allow_uninit);
        add_type_and_arrays!(self, u64, allow_uninit);
        add_type_and_arrays!(self, u128, allow_uninit);
        add_type_and_arrays!(self, usize, allow_uninit);

        add_type_and_arrays!(self, i8, allow_uninit);
        add_type_and_arrays!(self, i16, allow_uninit);
        add_type_and_arrays!(self, i32, allow_uninit);
        add_type_and_arrays!(self, i64, allow_uninit);
        add_type_and_arrays!(self, i128, allow_uninit);
        add_type_and_arrays!(self, isize, allow_uninit);

        add_type_and_arrays!(self, f32, allow_uninit);
        add_type_and_arrays!(self, f64, allow_uninit);

        add_type_and_arrays!(self, char, allow_uninit);

        add_type_and_arrays!(self, bool, allow_uninit);

        add_type_and_arrays!(self, String);
        add_type_and_arrays!(self, Box<str>);

        add_type_and_arrays!(self, Vec<()>);
    }

    #[cfg(feature = "uuid")]
    pub fn add_uuid_types(&mut self) {
        add_type_and_arrays!(self, uuid::Uuid, allow_uninit);
    }

    pub fn add_all_types(&mut self) {
        self.add_std_types();
        #[cfg(feature = "uuid")]
        self.add_uuid_types();
    }

    /// Serialization to a `serde_json::Value`.
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(&self.types)
    }

    /// Serialization to a `String`.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.types)
    }

    /// Serialization to a `String` with pretty printing.
    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.types)
    }
}

impl Default for StaticTypeResolver {
    /// Creates an empty resolver.
    fn default() -> Self {
        Self::new()
    }
}

impl TypeResolver for StaticTypeResolver {
    /// Gives the Rust type information for `T` by looking up loaded data.
    fn type_info<T>(&self) -> TypeInfo {
        let type_name = truc_type_name::<T>();
        self.types
            .get(&type_name)
            .unwrap_or_else(|| panic!("Could not resolve type {}", type_name))
            .info
            .clone()
    }

    /// Gives the dynamic type information  for `type_name` by looking up data.
    fn dynamic_type_info(&self, type_name: &str) -> DynamicTypeInfo {
        let type_name = truc_dynamic_type_name(type_name);
        self.types
            .get(&type_name)
            .unwrap_or_else(|| panic!("Could not resolve type {}", type_name))
            .clone()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_static_type_resolver() {
        let mut type_infos = StaticTypeResolver::default();

        type_infos.add_all_types();

        let json = type_infos.to_json_value().unwrap();
        type_infos.to_json_string().unwrap();
        type_infos.to_json_string_pretty().unwrap();

        let types = json.as_object().unwrap();

        for t in [
            "usize",
            "isize",
            "f32",
            "f64",
            "char",
            "bool",
            "String",
            "Box < str >",
            "Vec < () >",
        ] {
            assert!(types.contains_key(&t.to_owned()), "{}", t);
            assert!(types.contains_key(&format!("[{} ; 2]", t)), "[{} ; 2]", t);
            assert!(types.contains_key(&format!("[{} ; 10]", t)), "[{} ; 10]", t);
        }

        let name = assert_matches!(
            type_infos.type_info::<usize>(),
            TypeInfo {
                name,
                size: _,
                align: _
            } => name
        );
        assert_eq!(name, "usize");

        let name = assert_matches!(
            type_infos.dynamic_type_info("isize"),
            DynamicTypeInfo {
                info: TypeInfo {
                    name,
                    size: _,
                    align: _
                },
                allow_uninit: true,
            } => name
        );
        assert_eq!(name, "isize");

        let name = assert_matches!(
            type_infos.dynamic_type_info("String"),
            DynamicTypeInfo {
                info: TypeInfo {
                    name,
                    size: _,
                    align: _
                },
                allow_uninit: false,
            } => name
        );
        assert_eq!(name, "String");
    }

    #[test]
    fn test_type_added_twice() {
        let mut type_infos = StaticTypeResolver::default();

        type_infos.add_type::<usize>();

        let result = std::panic::catch_unwind(move || type_infos.add_type::<usize>());
        assert!(result.is_err());
    }

    #[test]
    fn test_type_added_twice_allow_uninit() {
        let mut type_infos = StaticTypeResolver::default();

        type_infos.add_type_allow_uninit::<usize>();

        let result = std::panic::catch_unwind(move || type_infos.add_type_allow_uninit::<usize>());
        assert!(result.is_err());
    }
}
