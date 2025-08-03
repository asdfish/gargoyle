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
    List(Box<Type>),
}

pub struct FnArgs {
    pub guile: bool,
    pub required: Vec<Box<Type>>,
    pub optional: Vec<Box<Type>>,
    pub rest: Option<Rest>,
}
impl FnArgs {
    /// Get the arity in `SCM` pointers.
    pub fn scm_arity(&self) -> usize {
        self.required.len()
            + self.optional.len()
            + self.rest.as_ref().map(|_| 1).unwrap_or_default()
    }
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
                                                && let Attribute {
                                                    meta:
                                                    Meta::NameValue(MetaNameValue {
                                                        value:
                                                        Expr::Lit(ExprLit {
                                                            lit: Lit::Str(val), ..
                                                        }),
                                                        ..
                                                    }),
                                                    ..
                                                } = attr
                                            {
                                                Some(val.value())
                                            } else {
                                                None
                                            }
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
                required,
                optional,
                rest,
            })
    }
}
