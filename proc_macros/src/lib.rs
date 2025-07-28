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

use {
    convert_case::{Case, Casing},
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::ToTokens,
    quote::quote,
    std::ffi::CString,
    syn::{
        Attribute, Expr, ExprLit, FnArg, Generics, Ident, ItemFn, Lit, Meta, MetaList,
        MetaNameValue, PatType, Path, Receiver, ReturnType, Signature, Token, Type,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        spanned::Spanned,
    },
};

#[derive(Default)]
struct Config {
    struct_ident: Option<String>,
    guile_ident: Option<(Span, String)>,
}
impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Punctuated::<Meta, Token!(,)>::parse_terminated(input).and_then(|args| {
            args.into_iter()
                .try_fold(Self::default(), |mut config, arg| match arg {
                    Meta::NameValue(MetaNameValue {
                        path,
                        value:
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(value),
                                ..
                            }),
                        ..
                    }) if path.is_ident("struct_ident") => {
                        config.struct_ident = Some(value.value());
                        Ok(config)
                    }
                    Meta::NameValue(MetaNameValue {
                        path,
                        value:
                            Expr::Lit(ExprLit {
                                lit: Lit::Str(value),
                                ..
                            }),
                        ..
                    }) if path.is_ident("guile_ident") => {
                        config.guile_ident = Some((value.span(), value.value()));
                        Ok(config)
                    }
                    Meta::NameValue(MetaNameValue { path, value, .. })
                        if path.is_ident("guile_ident") || path.is_ident("struct_ident") =>
                    {
                        Err(syn::Error::new(value.span(), "expected a string literal"))
                    }
                    Meta::NameValue(MetaNameValue { path, .. }) => {
                        Err(syn::Error::new(path.span(), "unknown argument"))
                    }
                    Meta::List(_) | Meta::Path(_) => Err(syn::Error::new(
                        arg.span(),
                        "argument can only be in the format of `key = val`",
                    )),
                })
        })
    }
}

#[derive(Default)]
struct Inputs {
    required: Vec<Box<Type>>,
    optional: Vec<Box<Type>>,
    rest: Option<Box<Type>>,
}
impl Inputs {
    pub fn push(&mut self, ty: Box<Type>) {
        (if self.optional.is_empty() {
            &mut self.required
        } else {
            &mut self.optional
        })
        .push(ty)
    }
}

fn attr_path(
    Attribute {
        meta:
            Meta::Path(path)
            | Meta::List(MetaList { path, .. })
            | Meta::NameValue(MetaNameValue { path, .. }),
        ..
    }: &Attribute,
) -> &Path {
    path
}

#[proc_macro_attribute]
pub fn guile_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse2::<Config>(args.into())
        .and_then(|config| syn::parse2::<ItemFn>(input.clone().into()).map(|input| (config, input)))
        .and_then(|(config, ItemFn { vis, sig, .. })| {
            let span = sig.span();
            let Signature {
                asyncness,
                unsafety,
                generics: Generics { params, .. },
                ident,
                inputs,
                output,
                ..
            } = sig;
            let output = match output {
                ReturnType::Default => quote!{ () },
                ReturnType::Type(_, ty) => quote! { #ty },
            };

            (asyncness.is_none() && unsafety.is_none() && params.is_empty())
                .then_some((config, vis, ident, inputs, output))
                .ok_or_else(|| {
                    syn::Error::new(
                        span,
                        "function signature cannot be async, unsafe, or generic",
                    )
                })
        })
        .and_then(|(config, vis, ident, inputs, output)| {
            inputs
                .into_iter()
                .map(|arg| match arg {
                    FnArg::Receiver(recv) => Err(recv),
                    FnArg::Typed(ty) => {
                        let span = ty.span();
                        let PatType { attrs, ty, .. } = ty;

                        Ok((span, attrs, ty))
                    }
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|recv| syn::Error::new(recv.span(), "function cannot be a method"))
                .and_then(|inputs| {
                    let mut inputs = inputs.into_iter();
                    let last = inputs.next_back();
                    let mut inputs = inputs.fold(Inputs::default(), |mut accum, (_, attrs, ty)| {
                        if attrs
                            .into_iter()
                            .any(|attr| attr_path(&attr).is_ident("optional"))
                        {
                            accum.optional.push(ty);
                        } else {
                            accum.push(ty);
                        }

                        accum
                   });
                    if let Some((span, attrs, ty)) = last {
                        #[derive(Default)]
                        struct Attrs {
                            optional: bool,
                            rest: bool,
                        }

                        let attrs = attrs.into_iter().fold(Attrs::default(), |mut accum, attr| {
                            let path = attr_path(&attr);
                            if path.is_ident("optional") {
                                accum.optional = true;
                            } else if path.is_ident("rest") {
                                accum.rest = true;
                            }
                            accum
                        });
                        match attrs {
                            Attrs {
                                optional: true,
                                rest: true,
                            } => Err(syn::Error::new(
                                span,
                                "the final argument cannot be both `optional` and `rest`",
                            )),
                            Attrs {
                                optional: true,
                                rest: false,
                            } => {
                                inputs.optional.push(ty);
                                Ok(())
                            }
                            Attrs {
                                optional: false,
                                rest: true,
                            } => {
                                inputs.rest = Some(ty);
                                Ok(())
                            }
                            Attrs {
                                optional: false,
                                rest: false,
                            } => {
                                inputs.push(ty);
                                Ok(())
                            }
                        }
                    } else {
                        Ok(())
                    }
                    .map(|_| inputs)
                })
                .map(|inputs| (config, vis, ident, inputs, output))
        })
        .and_then(
            |(
                Config {
                    guile_ident,
                    struct_ident,
                },
                vis,
                ident,
                Inputs {
                    required,
                    optional,
                    rest,
                },
                output,
            )| {
                let overflow_error = |error| syn::Error::new(Span::call_site(), format!("cannot have more arguments than `c_int::MAX`: {error}"));

                let required_len = required.len();
                let required_index = 0..i32::try_from(required_len).map_err(overflow_error)?;
                let required_idents = (0..required_len)
                    .map(|i| Ident::new(&format!("required_{i}"), Span::call_site()))
                    .collect::<Vec<_>>();

                let optional_len = optional.len();
                let optional_index = 0..i32::try_from(optional_len).map_err(overflow_error)?;
                let optional_idents = (0..optional_len)
                    .map(|i| Ident::new(&format!("optional_{i}"), Span::call_site()))
                    .collect::<Vec<_>>();

                let rest_is_some = rest.is_some();
                let rest = rest.into_iter().collect::<Vec<_>>();
                let rest_ident = rest_is_some
                    .then(|| Ident::new("rest", Span::call_site()))
                    .into_iter()
                    .collect::<Vec<_>>();

                let (span, guile_ident) = guile_ident
                    .unwrap_or_else(|| (ident.span(), ident.to_string().to_case(Case::Kebab)));
                let guile_ident = CString::new(guile_ident).map_err(|error| {
                    syn::Error::new(span, format!("identifier cannot have nul bytes: {error}"))
                })?;
                let struct_ident = Ident::new(
                    &struct_ident.unwrap_or_else(|| ident.to_string().to_case(Case::Pascal)),
                    Span::call_site(),
                );

                Ok(quote! {
                    #vis struct #struct_ident;
                    #[automatically_derived]
                    impl ::gargoyle::GuileFn for #struct_ident {
                        const ADDR: *mut ::core::ffi::c_void = {
                            extern "C" fn driver(
                                #(#required_idents: ::gargoyle::sys::SCM,)*
                                #(#optional_idents: ::gargoyle::sys::SCM,)*
                                #(#rest_ident: ::gargoyle::sys::SCM)*
                            ) -> ::gargoyle::sys::SCM {
                                #(const _: () = {
                                    const fn assert_scm_ty<T>()
                                    where
                                        T: ::gargoyle::ScmTy {}
                                    assert_scm_ty::<#required>()
                                };)*
                                const _: () = {
                                    const fn assert_scm_ty<T>()
                                    where
                                        T: ::gargoyle::ScmTy {}
                                    assert_scm_ty::<#output>()
                                };
                                #(const _: () = {
                                    const fn assert_optional_scm<T>()
                                    where
                                        T: ::gargoyle::OptionalScm {}
                                    assert_optional_scm::<#optional>()
                                };)*
                                #(const _: () = {
                                    const fn check_rest_scm<'a, T>()
                                    where
                                        T: ::gargoyle::RestScm<'a> {}
                                    check_rest_scm::<#rest>()
                                };)*

                                let output = #ident(
                                    #(unsafe { ::gargoyle::Scm::from_ptr(#required_idents) }
                                      .get::<#required>()
                                      .unwrap_or_else(|| {
                                          unsafe {
                                              ::gargoyle::sys::scm_wrong_type_arg_msg(
                                                  #guile_ident.as_ptr(),
                                                  #required_index,
                                                  #required_idents,
                                                  <#required as ::gargoyle::ScmTy>::TYPE_NAME.as_ptr(),
                                              )
                                          }
                                      }),)*
                                    #({
                                        type Inner = <#optional as ::gargoyle::OptionalScm>::Inner;
                                        let scm = unsafe { ::gargoyle::Scm::from_ptr(#optional_idents) };
                                        if scm.is::<()>() {
                                            ::core::option::Option::None
                                        } else if let ::core::option::Option::Some(#optional_idents) = scm.get::<Inner>() {
                                            ::core::option::Option::Some(#optional_idents)
                                        } else {
                                            unsafe {
                                                ::gargoyle::sys::scm_wrong_type_arg_msg(
                                                    #guile_ident.as_ptr(),
                                                    #optional_index,
                                                    #optional_idents,
                                                    <Inner as ::gargoyle::ScmTy>::TYPE_NAME.as_ptr(),
                                                )
                                            }
                                        }
                                    },)*
                                    #(unsafe { ::gargoyle::Scm::from_ptr(#rest_ident) },)*
                                );

                                unsafe { <#output as ::gargoyle::ScmTy>::construct(output, &::gargoyle::Api::new_unchecked()).as_ptr() }
                            }

                            driver as *mut ::core::ffi::c_void
                        };
                        const NAME: &::core::ffi::CStr = #guile_ident;

                        const REQUIRED: ::core::primitive::usize = #required_len;
                        const OPTIONAL: ::core::primitive::usize = #optional_len;
                        const REST: ::core::primitive::bool = #rest_is_some;
                    }
                })
            },
        )
        .map(|guile_fn| {
            [
                {
                    let mut item = syn::parse2::<ItemFn>(input.into()).expect("input was checked above");
                    let ItemFn { sig: Signature { ref mut inputs, .. }, .. } = item;
                    inputs.iter_mut()
                        .map(|arg| match arg {
                            FnArg::Receiver(Receiver { attrs, .. }) | FnArg::Typed(PatType { attrs, .. }) => attrs,
                        })
                        .for_each(|attrs| attrs.retain(|attr| {
                            let path = attr_path(&attr);
                            !(path.is_ident("optional") || path.is_ident("rest"))
                        }));
                    item.into_token_stream()
                },
                guile_fn
            ]
                .into_iter()
                .collect::<TokenStream2>()
        })
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
