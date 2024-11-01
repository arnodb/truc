use std::{collections::BTreeMap, prelude::v1::*};

use maplit::btreemap;
use syn::{
    visit_mut::{visit_type_path_mut, VisitMut},
    Path, PathSegment, Type, TypePath,
};

pub fn truc_type_name<T: ?Sized>() -> String {
    let std_type_name = std::any::type_name::<T>();
    truc_dynamic_type_name(std_type_name)
}

pub fn truc_dynamic_type_name(type_name: &str) -> String {
    let mut syn_type = syn::parse_str::<Type>(type_name).expect("syn type");
    rewrite_type(&mut syn_type);
    quote!(#syn_type).to_string()
}

fn rewrite_type(syn_type: &mut Type) {
    TypeRewriter.visit_type_mut(syn_type);
}

struct TypeRewriter;

impl VisitMut for TypeRewriter {
    fn visit_type_path_mut(&mut self, i: &mut TypePath) {
        let TypePath {
            qself: _,
            path: Path {
                leading_colon,
                segments,
            },
        } = i;
        if leading_colon.is_none() {
            let in_scope_types = PatternSegments(btreemap! {
                "alloc" => PatternSegments(btreemap!{
                    "boxed" => PatternSegments(btreemap!{
                        "Box" => PatternSegments(btreemap!{}),
                    }),
                    "string" => PatternSegments(btreemap!{
                        "String" => PatternSegments(btreemap!{}),
                    }),
                    "vec" => PatternSegments(btreemap!{
                        "Vec" => PatternSegments(btreemap!{}),
                    }),
                }),
                "core" => PatternSegments(btreemap!{
                    "option" => PatternSegments(btreemap!{
                        "Option" => PatternSegments(btreemap!{}),
                    }),
                    "result" => PatternSegments(btreemap!{
                        "Result" => PatternSegments(btreemap!{}),
                    }),
                }),
            });
            if match_segments(segments.iter(), &in_scope_types) {
                let seg = segments.pop().expect("segment").into_value();
                segments.clear();
                segments.push(seg);
            }
        }
        visit_type_path_mut(self, i);
    }
}

struct PatternSegments<'a>(BTreeMap<&'a str, Self>);

fn match_segments<'a>(
    mut segments: impl Iterator<Item = &'a PathSegment>,
    values: &'a PatternSegments<'a>,
) -> bool {
    if let Some(PathSegment {
        ident,
        arguments: _,
    }) = segments.next()
    {
        for (value, next) in &values.0 {
            if ident == value {
                return match_segments(segments, next);
            }
        }
        false
    } else {
        values.0.is_empty()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn usize_type_name() {
        assert_eq!(&*truc_type_name::<usize>(), "usize");
    }

    #[test]
    fn string_type_name() {
        assert_eq!(&*truc_type_name::<String>(), "String");
    }

    #[test]
    fn box_type_name() {
        assert_eq!(&*truc_type_name::<Box<String>>(), "Box < String >");
    }

    #[test]
    fn tuple_type_name() {
        assert_eq!(&*truc_type_name::<(usize, String)>(), "(usize , String)");
    }

    #[test]
    fn option_type_name() {
        assert_eq!(&*truc_type_name::<Option<u32>>(), "Option < u32 >");
        assert_eq!(&*truc_type_name::<Option<String>>(), "Option < String >");
    }

    #[test]
    fn vev_type_name() {
        assert_eq!(&*truc_type_name::<Vec<u32>>(), "Vec < u32 >");
        assert_eq!(&*truc_type_name::<Vec<String>>(), "Vec < String >");
    }

    #[test]
    fn result_type_name() {
        assert_eq!(
            &*truc_type_name::<Result<u32, String>>(),
            "Result < u32 , String >"
        );
    }

    #[test]
    fn array_type_name() {
        assert_eq!(&*truc_type_name::<[String; 42]>(), "[String ; 42]");
    }

    #[test]
    fn slice_type_name() {
        assert_eq!(&*truc_type_name::<[String]>(), "[String]");
    }
}
