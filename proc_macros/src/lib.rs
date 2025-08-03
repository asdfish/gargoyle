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
    proc_macro::TokenStream,
    proc_macro2::Span,
    quote::quote,
    syn::{FnArg, Ident, ItemFn, PatType, Receiver, Signature},
};

#[proc_macro_attribute]
pub fn guile_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse::<macro_args::Args>(args)
        .and_then(|args| syn::parse::<ItemFn>(input).map(|input| (args, input)))
        .and_then(|(args, mut input)| {
            let _config = macro_args::Config::new(args, &input);
            FnArgs::try_from(input.clone())
                .map(
                    |FnArgs {
                         guile,
                         required,
                         optional,
                         rest,
                     }| {
                        let ItemFn {
                            vis,
                            sig: Signature { generics, .. },
                            ..
                        } = input;

                        let required_len = required.len();
                        let optional_len = optional.len();
                        let has_rest = rest.is_some();

                        let required_idents = (0..required_len).map(|i| format!("required_{i}")).map(|i| Ident::new(&i, Span::call_site()));
                        let optional_idents = (0..optional_len).map(|i| format!("optional_{i}")).map(|i| Ident::new(&i, Span::call_site()));
                        let rest_ident = has_rest.then(|| Ident::new("rest", Span::call_site())).into_iter().collect::<Vec<_>>();
                        let keyword_static_idents = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some((0..keywords.len()).map(|i| format!("KEYWORD_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter().collect::<Vec<_>>();
                        let keyword_idents = rest.as_ref().and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some((0..keywords.len()).map(|i| format!("keyword_{i}")).map(|i| Ident::new(&i, Span::call_site())).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter().collect::<Vec<_>>();

                        let keyword_symbols = rest.and_then(|rest| match rest {
                            Rest::Keyword(keywords) => Some(keywords.into_iter().map(|(sym, _)| sym).collect::<Vec<_>>()),
                            Rest::List(_) => None,
                        }).into_iter().collect::<Vec<_>>();

                        quote! {
                            #vis struct #struct_ident;
                            unsafe impl #gargoyle_path::subr::GuileFn for #struct_ident {
                                const ADDR: *mut ::std::ffi::c_void = {
                                    unsafe extern "C" fn driver(
                                        #(#required_idents: #gargoyle_path::sys::SCM,)*
                                        #(#optional_idents: #gargoyle_path::sys::SCM,)*
                                        #(#rest_ident: #gargoyle_path::sys::SCM,)*
                                    ) -> #gargoyle_path::sys::SCM {
                                        #(#(static #keyword_static_idents = ::std::sync::LazyLock::new(|_| {
                                            let symbol = #keyword_symbols;
                                            unsafe { #gargoyle_path::sys::scm_from_utf8_symboln(symbol.as_bytes().as_ptr().cast(), symbol.len()) }
                                        });
                                        let mut #keyword_idents = unsafe { #gargoyle_path::sys::SCM_UNDEFINED };)*
                                        unsafe { #gargoyle_path::sys::scm_c_bind_arguments(); })*
                                        todo!()
                                    }

                                    driver as *mut ::std::ffi::c_void
                                };

                                const REQUIRED: ::std::primitive::usize = #required_len;
                                const OPTIONAL: ::std::primitive::usize = #optional_len;
                                const REST: ::std::primitive::bool = #has_rest;

                                const DOC: ::std::option::Option<&'static ::std::primitive::str> = #doc;
                                const NAME: &'static ::std::primitive::str = #guile_ident;
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
        })
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
