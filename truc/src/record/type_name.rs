pub fn truc_type_name<T: ?Sized>() -> String {
    let std_type_name = std::any::type_name::<T>();
    let mut syn_type = syn::parse_str::<syn::Type>(std_type_name).expect("syn type");
    rewrite_type(&mut syn_type);
    quote!(#syn_type).to_string()
}

pub fn truc_dynamic_type_name(type_name: &str) -> String {
    let mut syn_type = syn::parse_str::<syn::Type>(type_name).expect("syn type");
    rewrite_type(&mut syn_type);
    quote!(#syn_type).to_string()
}

fn rewrite_type(syn_type: &mut syn::Type) {
    #[cfg_attr(all(test, feature = "unstable"), deny(non_exhaustive_omitted_patterns))]
    match syn_type {
        syn::Type::Array(syn::TypeArray {
            bracket_token: _,
            elem,
            semi_token: _,
            len: _,
        }) => {
            rewrite_type(elem);
        }
        syn::Type::BareFn(_)
        | syn::Type::Group(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_) => {
            unimplemented!("{:?}", syn_type);
        }
        syn::Type::Paren(syn::TypeParen {
            paren_token: _,
            elem,
        }) => {
            rewrite_type(elem);
        }
        syn::Type::Path(syn::TypePath {
            qself: _,
            path: syn::Path {
                leading_colon,
                segments,
            },
        }) => {
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
            for seg in segments.iter_mut() {
                match &mut seg.arguments {
                    syn::PathArguments::None => {}
                    syn::PathArguments::AngleBracketed(args) => {
                        for arg in &mut args.args {
                            match arg {
                                syn::GenericArgument::Lifetime(_) => {}
                                syn::GenericArgument::Type(r#type) => rewrite_type(r#type),
                                syn::GenericArgument::Binding(_) => {}
                                syn::GenericArgument::Constraint(_) => {}
                                syn::GenericArgument::Const(_) => {}
                            }
                        }
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        for r#type in &mut args.inputs {
                            rewrite_type(r#type)
                        }
                    }
                }
            }
        }
        syn::Type::Ptr(_) | syn::Type::Reference(_) => {
            unimplemented!("{:?}", syn_type);
        }
        syn::Type::Slice(syn::TypeSlice {
            bracket_token: _,
            elem,
        }) => {
            rewrite_type(elem);
        }
        syn::Type::TraitObject(_) => {
            unimplemented!("{:?}", syn_type);
        }
        syn::Type::Tuple(syn::TypeTuple {
            paren_token: _,
            elems,
        }) => {
            for r#type in &mut elems.iter_mut() {
                rewrite_type(r#type)
            }
        }
        syn::Type::Verbatim(_) => {
            unimplemented!("{:?}", syn_type);
        }
        _ => {
            unimplemented!("{:?}", syn_type);
        }
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
