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
    syn::{
        Attribute, Expr, ExprLit, FnArg, ItemFn, Lit, Meta, MetaNameValue, Pat, PatIdent, PatType,
        Type, spanned::Spanned,
    },
};

pub enum Rest {
    /// Keyworded arguments so that you can call it with `:arg val`.
    Keyword(Vec<(String, Box<Type>)>),
    /// Represents the optional variadic arguments.
    ///
    /// This would be the `r` in `(lambda (. r) r)`
    // the list type may be useful one day
    #[expect(dead_code)]
    List(Box<Type>),
}

pub struct FnArgs {
    pub guile: bool,
    pub required: Vec<Type>,
    pub optional: Vec<Type>,
    pub rest: Option<Rest>,
}
impl TryFrom<ItemFn> for FnArgs {
    type Error = syn::Error;

    fn try_from(args: ItemFn) -> Result<Self, syn::Error> {
        #[derive(Clone, Copy)]
        enum RestTy {
            Keyword,
            List,
        }
        #[derive(Clone, Copy, Default)]
        enum State {
            #[default]
            Required,
            Optional,
            Rest(RestTy),
        }
        impl State {
            /// Return the a list of potential attributes that will change the current state.
            fn next_attrs(&self) -> Option<&'static [(&'static str, Self)]> {
                match self {
                    Self::Required => Some(&[
                        ("optional", Self::Optional),
                        ("keyword", Self::Rest(RestTy::Keyword)),
                        ("rest", Self::Rest(RestTy::List)),
                    ]),
                    Self::Optional => Some(&[
                        ("keyword", Self::Rest(RestTy::Keyword)),
                        ("rest", Self::Rest(RestTy::List)),
                    ]),
                    Self::Rest(_) => None,
                }
            }
        }

        let mut state = State::default();

        let mut args = args
            .sig
            .inputs
            .into_iter()
            .map(|arg| match arg {
                FnArg::Typed(arg) => Ok(arg),
                FnArg::Receiver(arg) => {
                    Err(syn::Error::new(arg.span(), "functions cannot be methods"))
                }
            })
            .peekable();

        let guile = args
            .next_if(|arg| {
                arg.as_ref()
                    .map(|PatType { attrs, .. }| {
                        attrs.iter().any(|attr| attr.path().is_ident("guile"))
                    })
                    .unwrap_or_default()
            })
            .is_some();
        args.map(|arg| arg
                .map(|arg| {
                    let PatType { ref attrs, .. } = arg;
                    if let Some(next_attrs) = state.next_attrs() {
                        if let Some((_, next_state)) =
                            next_attrs.iter().find(|(next_attr, _)| {
                                attrs.iter().any(|attr| attr.path().is_ident(next_attr))
                            })
                        {
                            state = *next_state;
                        }
                    }
                    (state, arg)
                }))
            .try_fold(
                (Vec::new(), Vec::new(), None),
                |(mut required, mut optional, mut rest), arg| {
                    arg.and_then(|(state, arg)| {
                        let PatType { attrs, pat, ty, .. } = arg;
                        match state {
                            State::Required => {
                                required.push(ty);
                                Ok(())
                            }
                            State::Optional => {
                                optional.push(ty);
                                Ok(())
                            }
                            State::Rest(RestTy::List) => {
                                if rest.is_none() {
                                    rest = Some(Rest::List(ty));
                                    Ok(())
                                } else {
                                    Err(syn::Error::new(ty.span(), "no more arguments may appear after using the `rest` attribute"))
                                }
                            }
                            State::Rest(RestTy::Keyword) => {
                                if let Pat::Ident(PatIdent { ident, .. }) = *pat.clone() {
                                    Some(ident.to_string())
                                } else {
                                    None
                                }
                                .map(|ident| ident.to_case(Case::Kebab))
                                    .or_else(|| {
                                        attrs.iter().find_map(|attr| {
                                            if attr.path().is_ident("keyword")
                                            {
                                                Some(attr)
                                            } else {
                                                None
                                            }
                                            .and_then(|attr| match attr {
                                                Attribute {
                                                    meta:
                                                    Meta::NameValue(MetaNameValue {
                                                        value:
                                                        Expr::Lit(ExprLit {
                                                            lit: Lit::Str(val), ..
                                                        }),
                                                        ..
                                                    }),
                                                    ..
                                                } => Some(val.value()),
                                                _ => None,
                                            })
                                        })
                                    })
                                    .ok_or_else(|| syn::Error::new(pat.span(), "Unable to create a keyword for this argument. Either bind the pattern to an identifier or use `#[keyword = \"keyword\"]` to set the keyword identifier."))
                                    .map(|ident| {
                                        match &mut rest {
                                            Some(Rest::List(_)) => unreachable!(),
                                            Some(Rest::Keyword(keywords)) => keywords,
                                            None => {
                                                rest = Some(Rest::Keyword(Vec::new()));
                                                rest.as_mut().map(|rest| match rest { Rest::Keyword(vec) => vec, _ => unreachable!("it should be set above") }).unwrap() 
                                            },
                                        }
                                        .push((ident, ty))
                                    })
                            }
                        }
                            .map(|_| (required, optional, rest))
                    })
                }
            )
            .map(|(required, optional, rest)| Self {
                guile,
                required: required.into_iter().map(|r| *r).collect(),
                optional: optional.into_iter().map(|r| *r).collect(),
                rest,
            })
    }
}
