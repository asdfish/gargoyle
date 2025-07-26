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

#[cfg(not(feature = "guile_3_0"))]
compile_error!("Neither of the `guile_*` features are selected.");

use std::{env, path::PathBuf, str};

fn compiler_args(
    pkg_config::Library {
        libs,
        link_paths,
        include_paths,
        ld_args,
        defines,
        ..
    }: pkg_config::Library,
) -> impl Iterator<Item = Result<String, str::Utf8Error>> {
    libs.into_iter()
        .map(|lib| format!("-l{lib}"))
        .map(Ok)
        .chain(link_paths.into_iter().map(|dir| {
            str::from_utf8(dir.as_os_str().as_encoded_bytes()).map(|dir| format!("-L{dir}"))
        }))
        .chain(include_paths.into_iter().map(|dir| {
            str::from_utf8(dir.as_os_str().as_encoded_bytes()).map(|dir| format!("-I{dir}"))
        }))
        .chain(
            ld_args
                .into_iter()
                .flatten()
                .map(|arg| format!("-Wl,{arg}"))
                .map(Ok),
        )
        .chain(
            defines
                .into_iter()
                .map(|(key, val)| match val {
                    Some(val) => format!("-D{key}={val}"),
                    None => key,
                })
                .map(Ok),
        )
}

#[cfg(feature = "guile_3_0")]
const GUILE_VERSION: &str = "guile-3.0";

fn main() {
    let args = pkg_config::Config::new().probe(GUILE_VERSION).unwrap();
    let libguile = args
        .include_paths
        .iter()
        .find_map(|path| {
            let path = path.join("libguile.h");
            path.is_file().then_some(path)
        })
        .unwrap();
    let args = compiler_args(args).collect::<Result<Vec<_>, _>>().unwrap();

    println!(
        "cargo:rerun-if-changed=build.rs
cargo:rerun-if-changed=src/reexports.h
cargo:rerun-if-changed=src/reexports.c"
    );

    cc::Build::new()
        .flags(args.clone())
        .file("src/reexports.c")
        .compile("reexports");
    bindgen::Builder::default()
        .clang_args(args)
        .header("src/reexports.h")
        .header(libguile.to_str().unwrap())
        .generate()
        .unwrap()
        .write_to_file(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("bindings.rs"))
        .unwrap();
}
