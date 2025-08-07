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

use std::{
    error::Error,
    ffi::OsStr,
    fmt::{self, Display, Formatter},
    io::{self, Write, stdout},
    process::Command,
};

// must be dynamically linked for lgpl
const PKG_CONFIG_ARGS: &[&str] = &["--cflags", "--libs", "--shared", "guile-3.0"];

pub fn pkg_config_guile() -> Result<Vec<u8>, PkgConfigError> {
    Command::new("pkg-config")
        .args(PKG_CONFIG_ARGS)
        .output()
        .map(|output| output.stdout)
        .map_err(PkgConfigError)
}

#[derive(Debug)]
pub struct PkgConfigError(io::Error);
impl Display for PkgConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "failed to execute `pkg-config")
            .and_then(|_| {
                PKG_CONFIG_ARGS
                    .iter()
                    .try_for_each(|arg| write!(f, " {arg}"))
            })
            .and_then(|_| write!(f, "`: {}", self.0))
    }
}
impl Error for PkgConfigError {}

fn die<T, U>(error: T) -> U
where
    T: Display,
{
    panic!("{error}")
}

fn main() {
    let mut stdout = stdout().lock();
    stdout
        .write_all(
            b"cargo:rerun-if-changed=build.rs
cargo:rerun-if-changed=src/reexports.h
cargo:rerun-if-changed=src/reexports.c\n",
        )
        .unwrap_or_else(die);

    pkg_config_guile()
        .unwrap_or_else(die)
        .split(u8::is_ascii_whitespace)
        .filter(|arg| !arg.is_empty())
        .try_fold(cc::Build::new(), |mut build, arg| {
            if let Some(link_dir) = arg.strip_prefix(b"-L") {
                stdout
                    .write_all(b"cargo:rustc-link-dir=")
                    .and_then(|_| stdout.write_all(link_dir))
                    .and_then(|_| stdout.write_all(b"\n"))
            } else if let Some(link_lib) = arg.strip_prefix(b"-l") {
                stdout
                    .write_all(b"cargo:rustc-link-lib=")
                    .and_then(|_| stdout.write_all(link_lib))
                    .and_then(|_| stdout.write_all(b"\n"))
            } else {
                build.flag_if_supported(unsafe { OsStr::from_encoded_bytes_unchecked(arg) });
                Ok(())
            }
            .map(|_| build)
        })
        .and_then(|build| stdout.flush().map(|_| drop(stdout)).map(|_| build))
        .unwrap_or_else(die)
        .file("src/reexports.c")
        .compile("reexports");
}
