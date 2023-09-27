use crate::record::type_name::truc_type_name;

#[derive(Clone, Debug)]
pub struct TypeInfo {
    pub name: String,
    pub size: usize,
    pub align: usize,
}

pub trait TypeResolver {
    fn type_info<T>(&self) -> TypeInfo;
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
}

impl<R> TypeResolver for &R
where
    R: TypeResolver,
{
    fn type_info<T>(&self) -> TypeInfo {
        R::type_info::<T>(self)
    }
}
