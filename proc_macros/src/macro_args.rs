// garguile - guile bindings for rust
// Copyright (C) 2025  Andrew Chi

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use {
    convert_case::{Case, Casing},
    proc_macro2::Span,
    std::{cell::LazyCell, ffi::CString},
    syn::{
        Attribute, Expr, ExprLit, Ident, ItemFn, Lit, LitCStr, LitStr, Meta, MetaNameValue, Path,
        Signature, Token,
        parse::{Parse, ParseStream},
        parse_quote,
        punctuated::Punctuated,
    },
};

mod keywords {
    use syn::custom_keyword;
    custom_keyword!(guile_ident);
    custom_keyword!(struct_ident);
    custom_keyword!(doc);
    custom_keyword!(garguile_root);

    custom_keyword!(r#false);
}

enum Key {
    GuileIdent,
    StructIdent,
    Doc,
    GarguileRoot,
}
impl Parse for Key {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lookahead = input.lookahead1();
        if lookahead.peek(keywords::guile_ident) {
            input
                .parse::<keywords::guile_ident>()
                .map(|_| Self::GuileIdent)
        } else if lookahead.peek(keywords::struct_ident) {
            input
                .parse::<keywords::struct_ident>()
                .map(|_| Self::StructIdent)
        } else if lookahead.peek(keywords::doc) {
            input.parse::<keywords::doc>().map(|_| Self::Doc)
        } else if lookahead.peek(keywords::garguile_root) {
            input
                .parse::<keywords::garguile_root>()
                .map(|_| Self::GarguileRoot)
        } else {
            Err(lookahead.error())
        }
    }
}

enum Arg {
    GuileIdent(CString),
    StructIdent(Ident),
    Doc(Option<String>),
    GarguileRoot(Path),
}
impl Parse for Arg {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Key::parse(input).and_then(|key| match key {
            Key::GuileIdent => <Token![=]>::parse(input)
                .and_then(|_| <LitCStr as Parse>::parse(input))
                .and_then(|lit| {
                    let string = lit.value();
                    if string.is_empty() {
                        Err(syn::Error::new(lit.span(), "identifiers cannot be empty"))
                    } else {
                        Ok(string)
                    }
                })
                .map(Self::GuileIdent),
            Key::StructIdent => <Token![=]>::parse(input)
                .and_then(|_| <Ident as Parse>::parse(input))
                .map(Self::StructIdent),
            Key::Doc => <Token![=]>::parse(input).and_then(|_| {
                let lookahead = input.lookahead1();
                if lookahead.peek(keywords::r#false) {
                    input.parse::<keywords::r#false>().map(|_| Self::Doc(None))
                } else if lookahead.peek(LitStr) {
                    input
                        .parse::<LitStr>()
                        .map(|doc| doc.value())
                        .map(Some)
                        .map(Self::Doc)
                } else {
                    Err(lookahead.error())
                }
            }),
            Key::GarguileRoot => <Token![=]>::parse(input)
                .and_then(|_| <Path as Parse>::parse(input))
                .map(Self::GarguileRoot),
        })
    }
}

pub struct Args(Punctuated<Arg, Token![,]>);
impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        Punctuated::parse_terminated(input).map(Self)
    }
}

pub struct Config {
    pub guile_ident: CString,
    pub struct_ident: Ident,
    pub doc: Option<String>,
    pub garguile_root: Path,
}
impl Config {
    pub fn new(
        args: Args,
        ItemFn {
            attrs,
            sig: Signature { ident, .. },
            ..
        }: &ItemFn,
    ) -> Self {
        let (guile_ident, struct_ident, doc, garguile_root) = args.0.into_iter().fold(
            (
                None,
                None,
                Some(
                    attrs
                        .iter()
                        .filter_map(|Attribute { meta, .. }| match meta {
                            Meta::NameValue(MetaNameValue {
                                path,
                                value:
                                    Expr::Lit(ExprLit {
                                        lit: Lit::Str(doc), ..
                                    }),
                                ..
                            }) if path.is_ident("doc") => Some(doc),
                            _ => None,
                        })
                        .map(|doc| doc.value())
                        .map(|mut doc| {
                            doc.push('\n');
                            doc
                        })
                        .collect::<String>()
                        .trim_end()
                        .to_string(),
                )
                .filter(|docs| !docs.is_empty()),
                None,
            ),
            |mut accum, arg| {
                match arg {
                    Arg::GuileIdent(ident) => accum.0 = Some(ident),
                    Arg::StructIdent(ident) => accum.1 = Some(ident),
                    Arg::Doc(doc) => accum.2 = doc,
                    Arg::GarguileRoot(root) => accum.3 = Some(root),
                }
                accum
            },
        );

        let ident = LazyCell::new(|| ident.to_string());
        Self {
            guile_ident: guile_ident
                .unwrap_or_else(|| CString::new(ident.to_case(Case::Kebab)).unwrap()),
            struct_ident: struct_ident
                .unwrap_or_else(|| Ident::new(&ident.to_case(Case::Pascal), Span::call_site())),
            doc,
            garguile_root: garguile_root.unwrap_or(parse_quote! { ::garguile }),
        }
    }
}
