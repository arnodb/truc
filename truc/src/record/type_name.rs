use syn::{
    visit_mut::{visit_type_path_mut, VisitMut},
    Path, TypePath,
};

pub fn truc_type_name<T: ?Sized>() -> String {
    let std_type_name = std::any::type_name::<T>();
    truc_dynamic_type_name(std_type_name)
}

pub fn truc_dynamic_type_name(type_name: &str) -> String {
    let mut syn_type = syn::parse_str::<syn::Type>(type_name).expect("syn type");
    rewrite_type(&mut syn_type);
    quote!(#syn_type).to_string()
}

fn rewrite_type(syn_type: &mut syn::Type) {
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
        let mut matched;
        matched = {
            leading_colon.is_none() && {
                let mut seg_iter = segments.iter_mut();
                match_segment(seg_iter.next(), "alloc", || {
                    match_segment(seg_iter.next(), "string", || {
                        match_segment(seg_iter.next(), "String", || seg_iter.next().is_none())
                    })
                })
            }
        };
        if matched {
            let mut seg = segments.pop().expect("segment").into_value();
            seg.ident = syn::Ident::new("String", proc_macro2::Span::call_site());
            segments.clear();
            segments.push(seg);
        }
        if !matched {
            matched = {
                leading_colon.is_none() && {
                    let mut seg_iter = segments.iter_mut();
                    match_segment(seg_iter.next(), "alloc", || {
                        match_segment(seg_iter.next(), "boxed", || {
                            match_segment_with_arguments(seg_iter.next(), "Box", || {
                                seg_iter.next().is_none()
                            })
                        })
                    })
                }
            };
            if matched {
                let mut seg = segments.pop().expect("segment").into_value();
                seg.ident = syn::Ident::new("Box", proc_macro2::Span::call_site());
                segments.clear();
                segments.push(seg);
            }
        }
        if !matched {
            matched = leading_colon.is_none() && {
                let mut seg_iter = segments.iter_mut();
                match_segment(seg_iter.next(), "core", || {
                    match_segment(seg_iter.next(), "option", || {
                        match_segment_with_arguments(seg_iter.next(), "Option", || {
                            seg_iter.next().is_none()
                        })
                    })
                })
            };
            if matched {
                let mut seg = segments.pop().expect("segment").into_value();
                seg.ident = syn::Ident::new("Option", proc_macro2::Span::call_site());
                segments.clear();
                segments.push(seg);
            }
        }
        if !matched {
            matched = leading_colon.is_none() && {
                let mut seg_iter = segments.iter_mut();
                match_segment(seg_iter.next(), "core", || {
                    match_segment(seg_iter.next(), "result", || {
                        match_segment_with_arguments(seg_iter.next(), "Result", || {
                            seg_iter.next().is_none()
                        })
                    })
                })
            };
            if matched {
                let mut seg = segments.pop().expect("segment").into_value();
                seg.ident = syn::Ident::new("Result", proc_macro2::Span::call_site());
                segments.clear();
                segments.push(seg);
            }
        }
        visit_type_path_mut(self, i);
    }
}

fn match_segment<F>(segment: Option<&mut syn::PathSegment>, value: &str, f: F) -> bool
where
    F: FnOnce() -> bool,
{
    if let Some(syn::PathSegment {
        ident,
        arguments: syn::PathArguments::None,
    }) = segment
    {
        if ident == &mut syn::Ident::new(value, proc_macro2::Span::call_site()) {
            f()
        } else {
            false
        }
    } else {
        false
    }
}

fn match_segment_with_arguments<F>(
    segment: Option<&mut syn::PathSegment>,
    value: &str,
    f: F,
) -> bool
where
    F: FnOnce() -> bool,
{
    if let Some(syn::PathSegment {
        ident,
        arguments:
            syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                colon2_token: None,
                lt_token: _,
                args: _,
                gt_token: _,
            }),
    }) = segment
    {
        if ident == &mut syn::Ident::new(value, proc_macro2::Span::call_site()) {
            f()
        } else {
            false
        }
    } else {
        false
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
