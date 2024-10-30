use std::collections::{btree_map::Entry, BTreeMap};

use serde::{Deserialize, Serialize};

use crate::record::type_name::{truc_dynamic_type_name, truc_type_name};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub align: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicTypeInfo {
    pub info: TypeInfo,
    pub allow_uninit: bool,
}

pub trait TypeResolver {
    fn type_info<T>(&self) -> TypeInfo;

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

pub struct HostTypeResolver;

impl TypeResolver for HostTypeResolver {
    fn type_info<T>(&self) -> TypeInfo {
        TypeInfo {
            name: truc_type_name::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }

    fn dynamic_type_info(&self, _type_name: &str) -> DynamicTypeInfo {
        unimplemented!("HostTypeResolver cannot resolve dynamic types")
    }
}

#[derive(Debug, From, Serialize, Deserialize)]
pub struct StaticTypeResolver {
    types: BTreeMap<String, DynamicTypeInfo>,
}

impl StaticTypeResolver {
    pub fn new() -> Self {
        Self {
            types: BTreeMap::new(),
        }
    }

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

    pub fn add_std_types(&mut self) {
        macro_rules! add_type {
            ($type:ty) => {
                self.add_type::<$type>();
                self.add_type::<Option<$type>>();
            };
            ($type:ty, allow_uninit) => {
                self.add_type_allow_uninit::<$type>();
                self.add_type_allow_uninit::<Option<$type>>();
            };
        }
        macro_rules! add_type_and_arrays {
            ($type:ty) => {
                add_type!($type);
                add_type!([$type; 1]);
                add_type!([$type; 2]);
                add_type!([$type; 3]);
                add_type!([$type; 4]);
                add_type!([$type; 5]);
                add_type!([$type; 6]);
                add_type!([$type; 7]);
                add_type!([$type; 8]);
                add_type!([$type; 9]);
                add_type!([$type; 10]);
            };
            ($type:ty, allow_uninit) => {
                add_type!($type, allow_uninit);
                add_type!([$type; 1], allow_uninit);
                add_type!([$type; 2], allow_uninit);
                add_type!([$type; 3], allow_uninit);
                add_type!([$type; 4], allow_uninit);
                add_type!([$type; 5], allow_uninit);
                add_type!([$type; 6], allow_uninit);
                add_type!([$type; 7], allow_uninit);
                add_type!([$type; 8], allow_uninit);
                add_type!([$type; 9], allow_uninit);
                add_type!([$type; 10], allow_uninit);
            };
        }
        add_type_and_arrays!(u8, allow_uninit);
        add_type_and_arrays!(u16, allow_uninit);
        add_type_and_arrays!(u32, allow_uninit);
        add_type_and_arrays!(u64, allow_uninit);
        add_type_and_arrays!(u128, allow_uninit);
        add_type_and_arrays!(usize, allow_uninit);

        add_type_and_arrays!(i8, allow_uninit);
        add_type_and_arrays!(i16, allow_uninit);
        add_type_and_arrays!(i32, allow_uninit);
        add_type_and_arrays!(i64, allow_uninit);
        add_type_and_arrays!(i128, allow_uninit);
        add_type_and_arrays!(isize, allow_uninit);

        add_type_and_arrays!(f32, allow_uninit);
        add_type_and_arrays!(f64, allow_uninit);

        add_type_and_arrays!(char, allow_uninit);

        add_type_and_arrays!(bool, allow_uninit);

        add_type_and_arrays!(String);
        add_type_and_arrays!(Box<str>);

        add_type_and_arrays!(Vec<()>);
    }

    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(&self.types)
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.types)
    }

    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.types)
    }
}

impl Default for StaticTypeResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeResolver for StaticTypeResolver {
    fn type_info<T>(&self) -> TypeInfo {
        let type_name = truc_type_name::<T>();
        self.types
            .get(&type_name)
            .unwrap_or_else(|| panic!("Could not resolve type {}", type_name))
            .info
            .clone()
    }

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

        type_infos.add_std_types();

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
