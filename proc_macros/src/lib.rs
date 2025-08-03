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

use {fn_args::FnArgs, proc_macro::TokenStream, syn::ItemFn};

#[proc_macro_attribute]
pub fn guile_fn(args: TokenStream, input: TokenStream) -> TokenStream {
    syn::parse::<macro_args::Args>(args)
        .and_then(|args| syn::parse::<ItemFn>(input).map(|input| (args, input)))
        .and_then(|(args, mut input)| {
            let _config = macro_args::Config::new(args, &input);
            let _fn_args = FnArgs::try_from(&mut input)?;
            // input.signature
            //     .inputs
            //     .iter_mut()

            todo!()
        })
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
