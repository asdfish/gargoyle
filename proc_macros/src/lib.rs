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
    proc_macro::TokenStream,
    syn::{
        Expr, ExprLit, ItemFn, Lit, Meta, MetaNameValue, Token,
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        spanned::Spanned,
    },
};

#[derive(Default)]
struct Config {
    struct_ident: Option<String>,
    guile_ident: Option<String>,
    rest: bool,
}
impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Punctuated::<Meta, Token!(,)>::parse_terminated(input).and_then(|args| {
            args.into_iter()
                .try_fold(Self::default(), |mut config, arg| match arg {
                    Meta::Path(path) if path.is_ident("rest") => {
                        config.rest = true;
                        Ok(config)
                    }
                    Meta::List(_) => Err(syn::Error::new(
                        arg.span(),
                        "argument can only be `key = val`, or `path`",
                    )),
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
                        config.guile_ident = Some(value.value());
                        Ok(config)
                    }
                    Meta::NameValue(MetaNameValue { path, value, .. })
                        if path.is_ident("guile_ident") || path.is_ident("struct_ident") =>
                    {
                        Err(syn::Error::new(value.span(), "expected string literal"))
                    }
                    Meta::Path(path) | Meta::NameValue(MetaNameValue { path, .. }) => {
                        Err(syn::Error::new(path.span(), "unknown argument"))
                    }
                })
        })
    }
}

#[proc_macro_attribute]
pub fn guile_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse2::<Config>(args.into())
        .and_then(|config| syn::parse2::<ItemFn>(input.into()).map(|input| (config, input)))
        .map(|_| todo!())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
