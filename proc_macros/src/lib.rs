// gargoyle - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

mod fn_args;
mod macro_args;

use {
    crate::{
        fn_args::{FnArgs, Rest},
        macro_args::Config,
    },
    convert_case::{Case, Casing},
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::quote,
    std::{borrow::Cow, ffi::CString, iter},
    syn::{
        Attribute, DeriveInput, Expr, ExprLit, ExprPath, FnArg, GenericParam, Generics, Ident,
        ItemFn, Lifetime, LifetimeParam, Lit, LitCStr, MetaNameValue, PatType, Path, Receiver,
        Signature, parse_quote, spanned::Spanned,
    },
};

#[proc_macro_attribute]
pub fn guile_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse::<macro_args::Args>(args)
        .and_then(|args| syn::parse::<ItemFn>(input).map(|input| (args, input)))
        .and_then(|(args, mut input)| {
            let Config {
                guile_ident,
                struct_ident,
                doc,
                gargoyle_root,
            } = Config::new(args, &input);
            FnArgs::try_from(input.clone())
                .map(
                    |FnArgs {
                         guile,
                         required,
                         optional,
                         rest,
                     }| {
                        let ItemFn {
                            ref vis,
                            sig: Signature { ref ident, .. },
                            ..
                        } = input;

                        let doc = doc.map(|doc| quote! { Some(#doc) }).unwrap_or_else(|| quote! { None });

                        let required_len = required.len();
                        let optional_len = optional.len();
                        let has_rest = rest.is_some();

                        let required_idxs = 0..required_len;
                        let required_idents = (0..required_len).map(|i| format!("required_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>();

                        let optional_idxs = required_len..required_len + optional_len;
                        let optional_idents = (0..optional_len).map(|i| format!("optional_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>();
                        let rest_ident = has_rest.then(|| Ident::new("rest", Span::call_site())).into_iter().collect::<Vec<_>>();

                        let keyword_idxs = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some((required_len + optional_len..required_len + optional_len + keywords.len()).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter();
                        let keyword_static_idents = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some((0..keywords.len()).map(|i| format!("KEYWORD_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter();
                        let keyword_idents = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some((0..keywords.len()).map(|i| format!("keyword_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter().collect::<Vec<_>>();
                        let keyword_symbols = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some(keywords.iter().map(|(sym, _)| sym).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter();

                        let guile = guile.then(|| quote! { guile, });

                        let rest_idx = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(_) => None,
                            Rest::List(_) => Some(optional_len + required_len),
                        }).into_iter();
                        let rest_list = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(_) => None,
                            Rest::List(_) => Some(quote! { rest }),
                        }).into_iter();
                        let rest_enabled_ident = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(_) => None,
                            Rest::List(_) => Some(quote! { rest }),
                        }).into_iter();

                        quote! {
                            #vis struct #struct_ident;
                            unsafe impl #gargoyle_root::subr::GuileFn for #struct_ident {
                                const ADDR: *mut ::std::ffi::c_void = {
                                    unsafe extern "C" fn driver(
                                        #(#required_idents: #gargoyle_root::sys::SCM,)*
                                        #(#optional_idents: #gargoyle_root::sys::SCM,)*
                                        #(#rest_ident: #gargoyle_root::sys::SCM,)*
                                    ) -> #gargoyle_root::sys::SCM {
                                        let guile = unsafe { #gargoyle_root::Guile::new_unchecked_ref() };

                                        #(let #required_idents = ::std::mem::ManuallyDrop::new(#gargoyle_root::scm::TryFromScm::from_scm_or_throw(#gargoyle_root::scm::Scm::from_ptr(#required_idents, guile), #guile_ident, #required_idxs, guile));)*
                                        #(let #optional_idents = <::std::option::Option<_> as #gargoyle_root::scm::TryFromScm>::from_scm_or_throw(#gargoyle_root::scm::Scm::from_ptr(#optional_idents, guile), #guile_ident, #optional_idxs, guile).map(::std::mem::ManuallyDrop::new);)*
                                        #(#(static #keyword_static_idents: ::std::sync::LazyLock<::std::sync::atomic::AtomicPtr<#gargoyle_root::sys::scm_unused_struct>> = ::std::sync::LazyLock::new(|| {
                                            const SYMBOL: &'static ::std::primitive::str = #keyword_symbols;
                                            unsafe { #gargoyle_root::sys::scm_symbol_to_keyword(#gargoyle_root::sys::scm_from_utf8_symboln(SYMBOL.as_bytes().as_ptr().cast(), SYMBOL.len()))}.into()
                                        });
                                        let mut #keyword_idents = unsafe { #gargoyle_root::sys::SCM_UNDEFINED };)*
                                        unsafe { #gargoyle_root::sys::scm_c_bind_keyword_arguments(
                                            #guile_ident.as_ptr(), #rest_ident, 0,
                                            #(#keyword_static_idents.load(::std::sync::atomic::Ordering::SeqCst), &raw mut #keyword_idents,)*
                                            #gargoyle_root::sys::SCM_UNDEFINED,
                                        ); }
                                        #(let #keyword_idents = <::std::option::Option<_> as #gargoyle_root::scm::TryFromScm>::from_scm_or_throw(#gargoyle_root::scm::Scm::from_ptr(#keyword_idents, guile), #guile_ident, #keyword_idxs, guile).map(::std::mem::ManuallyDrop::new);)*)*
                                        #(let #rest_ident: ::std::mem::ManuallyDrop<#gargoyle_root::collections::list::List<_>> = ::std::mem::ManuallyDrop::new(#gargoyle_root::scm::TryFromScm::from_scm_or_throw(#gargoyle_root::scm::Scm::from_ptr(#rest_list, guile), #guile_ident, #rest_idx, guile));)*

                                        let ret = #ident(
                                            #guile
                                            #(&#required_idents,)*
                                            #(#optional_idents.as_deref(),)*
                                            #(#(#keyword_idents.as_deref(),)*)*
                                            #(&#rest_enabled_ident)*
                                        );
                                        #gargoyle_root::scm::ToScm::to_scm(ret, guile).as_ptr()
                                    }

                                    driver as *mut ::std::ffi::c_void
                                };

                                const REQUIRED: ::std::primitive::usize = #required_len;
                                const OPTIONAL: ::std::primitive::usize = #optional_len;
                                const REST: ::std::primitive::bool = #has_rest;

                                const DOC: ::std::option::Option<&'static ::std::primitive::str> = #doc;
                                const NAME: &'static ::std::ffi::CStr = #guile_ident;
                            }
                        }
                    },
                )
                .inspect(|_| {
                    input
                        .sig
                        .inputs
                        .iter_mut()
                        .map(
                            |(FnArg::Receiver(Receiver { attrs, .. })
                             | FnArg::Typed(PatType { attrs, .. }))| {
                                attrs
                            },
                        )
                        .for_each(|attrs| {
                            attrs.retain(|attr| {
                                !(attr.path().is_ident("guile")
                                    || attr.path().is_ident("optional")
                                    || attr.path().is_ident("rest")
                                    || attr.path().is_ident("keyword"))
                            })
                        });
                })
                .map(|tokens| quote! { #tokens #input })
        })
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn get_last_attr<'a, C, I, F, T>(
    attrs: &'a C,
    ident: &str,
    mut filter: F,
    default: T,
) -> Result<Cow<'a, T>, syn::Error>
where
    &'a C: IntoIterator<Item = &'a Attribute, IntoIter = I>,
    I: DoubleEndedIterator + Iterator<Item = &'a Attribute>,
    F: FnMut(&'a Expr) -> Result<&'a T, syn::Error>,
    T: Clone,
{
    attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident(ident))
        .map(|attr| {
            attr.meta
                .require_name_value()
                .and_then(|MetaNameValue { value, .. }| filter(value))
                .map(Cow::Borrowed)
        })
        .next_back()
        .unwrap_or(Ok(Cow::Owned(default)))
}
fn gargoyle_root<'a, C, I>(attrs: &'a C) -> Result<Cow<'a, Path>, syn::Error>
where
    &'a C: IntoIterator<Item = &'a Attribute, IntoIter = I>,
    I: DoubleEndedIterator + Iterator<Item = &'a Attribute>,
{
    get_last_attr(
        attrs,
        "gargoyle_root",
        |expr| match expr {
            Expr::Path(ExprPath { path, .. }) => Ok(path),
            expr => Err(syn::Error::new(
                expr.span(),
                "expected path: `gargoyle_root = ::foo`",
            )),
        },
        parse_quote! { ::gargoyle },
    )
}
fn guile_mode_lt<'a, C, I>(attrs: &'a C) -> Result<Cow<'a, Ident>, syn::Error>
where
    &'a C: IntoIterator<Item = &'a Attribute, IntoIter = I>,
    I: DoubleEndedIterator + Iterator<Item = &'a Attribute>,
{
    get_last_attr(
        attrs,
        "guile_mode_lt",
        |expr| {
            match expr {
                Expr::Path(ExprPath { path, .. }) => path.get_ident(),
                _ => None,
            }
            .ok_or_else(|| {
                syn::Error::new(expr.span(), "expected identifier: `guile_mode_lt = foo`")
            })
        },
        parse_quote! { gm },
    )
}

#[proc_macro_derive(ForeignObject, attributes(gargoyle_root, ty_name))]
pub fn foreign_object(input: TokenStream) -> TokenStream {
    syn::parse::<DeriveInput>(input)
        .and_then(
            |DeriveInput {
                 attrs,
                 ident,
                 generics,
                 ..
             }| {
                let ty_name_str = ident.to_string().to_case(Case::Kebab);
                gargoyle_root(&attrs)
                    .and_then(|gargoyle_root| {
                        get_last_attr(
                            &attrs,
                            "ty_name",
                            |expr| {
                                match expr {
                                    Expr::Lit(ExprLit { lit: Lit::CStr(lit), .. }) => Ok(lit),
                                    expr => Err(syn::Error::new(expr.span(), "expected literal c string: `ty_name = c\"foo\"`")),
                                }
                            },
                            LitCStr::new(&CString::new(ty_name_str).expect("rust identifiers cannot have null characters which would make this unreachable"), Span::call_site()),
                        )
                            .map(|ty_name| (gargoyle_root, ty_name))
                    })
                    .map(|(gargoyle_root, ty_name_cstr)| {
                        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
                        let ty_name_str = ty_name_cstr.value();
                        let ty_name_str = ty_name_str.to_string_lossy();

                        quote! {
                            impl #impl_generics #gargoyle_root::foreign_object::ForeignObject for #ident #ty_generics
                            #where_clause
                            {
                                const TYPE_NAME: &::std::ffi::CStr = #ty_name_cstr;
                                unsafe fn get_or_create_type() -> #gargoyle_root::sys::SCM {
                                    static OBJECT_TYPE: ::std::sync::LazyLock<::std::sync::atomic::AtomicPtr<#gargoyle_root::sys::scm_unused_struct>>
                                        = ::std::sync::LazyLock::new(|| {
                                            let guile = unsafe { #gargoyle_root::Guile::new_unchecked_ref() };
                                            let name = #gargoyle_root::symbol::Symbol::from_str(#ty_name_str, guile);
                                            unsafe {
                                                #gargoyle_root::sys::scm_make_foreign_object_type(
                                                    #gargoyle_root::reference::ReprScm::to_ptr(name),
                                                    #gargoyle_root::foreign_object::slots(),
                                                    ::std::option::Option::None,
                                                )
                                            }.into()
                                        });

                                    OBJECT_TYPE.load(::std::sync::atomic::Ordering::Acquire)
                                }
                            }
                        }
                    })
            },
        )
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn add_lifetime(lt: Lifetime, mut generics: Generics) -> Generics {
    if !generics.params.iter().any(|param| {
        matches!(param, GenericParam::Lifetime(LifetimeParam {
                lifetime: Lifetime { ident, .. }, ..
            }) if *ident == lt.ident)
    }) {
        generics.params = iter::once(GenericParam::Lifetime(LifetimeParam::new(lt)))
            .chain(generics.params.clone())
            .collect();
    }

    generics
}

#[proc_macro_derive(ToScm, attributes(gargoyle_root, guile_mode_lt))]
pub fn to_scm(input: TokenStream) -> TokenStream {
    syn::parse::<DeriveInput>(input)
        .and_then(
            |DeriveInput {
                 attrs,
                 ident,
                 generics,
                 ..
             }| {
                gargoyle_root(&attrs)
                    .and_then(|gargoyle_root| guile_mode_lt(&attrs)
                              .map(|ident| Lifetime {
                                  apostrophe: Span::call_site(),
                                  ident: ident.into_owned(),
                              })
                              .map(|gm| (gargoyle_root, gm)))
                    .map(|(gargoyle_root, gm)| {
                        let (_, ty_generics, _) = generics.split_for_impl();
                        let ty_generics = quote! { #ty_generics };

                        let generics = add_lifetime(gm.clone(), generics);
                        let (impl_generics, _, where_clause) = generics.split_for_impl();

                        let where_clause = where_clause.cloned()
                            .map(|mut clause| {
                                clause.predicates.push(parse_quote! { Self: #gargoyle_root::foreign_object::ForeignObject });
                                clause
                            })
                            .unwrap_or_else(|| parse_quote! {
                                where
                                    Self: #gargoyle_root::foreign_object::ForeignObject,
                            });
                        quote! {
                            impl #impl_generics #gargoyle_root::scm::ToScm<#gm> for #ident #ty_generics
                            #where_clause
                            {
                                fn to_scm(self, guile: &'gm #gargoyle_root::Guile) -> #gargoyle_root::scm::Scm<'gm> {
                                    // we don't need to care about panicking or dynwind since the pointer is garbage collected
                                    let ptr = #gargoyle_root::alloc::allocator_api2::boxed::Box::into_raw(
                                        #gargoyle_root::alloc::allocator_api2::boxed::Box::new_in(self, #gargoyle_root::alloc::GcAllocator::from(guile))
                                    );
                                    #gargoyle_root::scm::Scm::from_ptr(unsafe { #gargoyle_root::sys::scm_make_foreign_object_1(<Self as #gargoyle_root::foreign_object::ForeignObject>::get_or_create_type(), ptr.cast()) }, guile)
                                }
                            }
                        }
                    })
            },
        )
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(TryFromScm, attributes(gargoyle_root, guile_mode_lt))]
pub fn try_from_scm(input: TokenStream) -> TokenStream {
    syn::parse::<DeriveInput>(input)
        .and_then(
            |DeriveInput {
                 attrs,
                 ident,
                 generics,
                 ..
             }| {
                gargoyle_root(&attrs)
                    .and_then(|gargoyle_root| {
                        guile_mode_lt(&attrs)
                            .map(|ident| Lifetime {
                                apostrophe: Span::call_site(),
                                ident: ident.into_owned(),
                            })
                            .map(|gm| (gargoyle_root, gm))
                    })
                    .map(|(gargoyle_root, gm)| {
                        let (_, ty_generics, _) = generics.split_for_impl();
                        let ty_generics = quote! { #ty_generics };

                        let generics = add_lifetime(gm.clone(), generics);
                        let (impl_generics, _, where_clause) = generics.split_for_impl();
                        let where_clause = where_clause.cloned()
                            .map(|mut clause| {
                                clause.predicates.push(parse_quote! { Self: #gargoyle_root::foreign_object::ForeignObject });
                                clause
                            })
                            .unwrap_or_else(|| parse_quote! {
                                where
                                    Self: #gargoyle_root::foreign_object::ForeignObject,
                            });

                        quote! {
                            impl #impl_generics #gargoyle_root::scm::TryFromScm<#gm> for #ident #ty_generics
                            #where_clause
                            {
                                fn type_name() -> ::std::borrow::Cow<'static, ::std::ffi::CStr> {
                                    ::std::borrow::Cow::Borrowed(<Self as #gargoyle_root::foreign_object::ForeignObject>::TYPE_NAME)
                                }

                                fn predicate(scm: &#gargoyle_root::scm::Scm<#gm>, _: &#gm #gargoyle_root::Guile) -> bool {
                                    let b = unsafe {
                                        #gargoyle_root::sys::SCM_IS_A_P(
                                            scm.as_ptr(),
                                            <Self as #gargoyle_root::foreign_object::ForeignObject>::get_or_create_type(),
                                        )
                                    };
                                    b != 0
                                }

                                unsafe fn from_scm_unchecked(scm: #gargoyle_root::scm::Scm<#gm>, _: &#gm #gargoyle_root::Guile) -> Self {
                                    let ptr = unsafe {
                                        #gargoyle_root::sys::scm_foreign_object_ref(
                                            scm.as_ptr(),
                                            0,
                                        )
                                    }.cast::<Self>();
                                    if ptr.is_null() {
                                        ::std::panic!("unexpected null pointer")
                                    } else if ptr.is_aligned() {
                                        unsafe { ptr.read() }
                                    } else {
                                        unsafe { ptr.read_unaligned() }
                                    }
                                }
                            }
                        }
                    })
            },
        )
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
