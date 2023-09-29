use crate::record::type_name::{truc_dynamic_type_name, truc_type_name};
use serde::{Deserialize, Serialize};
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub align: usize,
}

pub trait TypeResolver {
    fn type_info<T>(&self) -> TypeInfo;

    fn dynamic_type_info(&self, type_name: &str) -> TypeInfo;
}

impl<R> TypeResolver for &R
where
    R: TypeResolver,
{
    fn type_info<T>(&self) -> TypeInfo {
        R::type_info::<T>(self)
    }

    fn dynamic_type_info(&self, type_name: &str) -> TypeInfo {
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

    fn dynamic_type_info(&self, _type_name: &str) -> TypeInfo {
        unimplemented!("HostTypeResolver cannot resolve dynamic types")
    }
}

#[derive(Debug, From)]
pub struct StaticTypeResolver {
    types: BTreeMap<String, TypeInfo>,
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
                vacant.insert(TypeInfo {
                    name: type_name,
                    size: std::mem::size_of::<T>(),
                    align: std::mem::align_of::<T>(),
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
        macro_rules! add_type_and_arrays {
            ($type:ty) => {
                self.add_type::<$type>();
                self.add_type::<[$type; 1]>();
                self.add_type::<[$type; 2]>();
                self.add_type::<[$type; 3]>();
                self.add_type::<[$type; 4]>();
                self.add_type::<[$type; 5]>();
                self.add_type::<[$type; 6]>();
                self.add_type::<[$type; 7]>();
                self.add_type::<[$type; 8]>();
                self.add_type::<[$type; 9]>();
                self.add_type::<[$type; 10]>();
            };
        }
        add_type_and_arrays!(u8);
        add_type_and_arrays!(u16);
        add_type_and_arrays!(u32);
        add_type_and_arrays!(u64);
        add_type_and_arrays!(u128);
        add_type_and_arrays!(usize);

        add_type_and_arrays!(i8);
        add_type_and_arrays!(i16);
        add_type_and_arrays!(i32);
        add_type_and_arrays!(i64);
        add_type_and_arrays!(i128);
        add_type_and_arrays!(isize);

        add_type_and_arrays!(f32);
        add_type_and_arrays!(f64);

        add_type_and_arrays!(char);

        add_type_and_arrays!(bool);

        add_type_and_arrays!(String);
        add_type_and_arrays!(Box<str>);

        add_type_and_arrays!(Vec<()>);
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
            .clone()
    }

    fn dynamic_type_info(&self, type_name: &str) -> TypeInfo {
        let type_name = truc_dynamic_type_name(type_name);
        self.types
            .get(&type_name)
            .unwrap_or_else(|| panic!("Could not resolve type {}", type_name))
            .clone()
    }
}
