# Simple fetch and unroll .tag.gz archives

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-brightgreen.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io Package](https://img.shields.io/crates/v/fetch_unroll.svg?style=popout)](https://crates.io/crates/fetch_unroll)
[![Docs.rs API Docs](https://docs.rs/fetch_unroll/badge.svg)](https://docs.rs/fetch_unroll)
[![Travis-CI Status](https://travis-ci.com/katyo/fetch_unroll.svg?branch=master)](https://travis-ci.com/katyo/fetch_unroll)

Simple functions intended to use in __Rust__ `build.rs` scripts for tasks which related to fetching from _HTTP_ and unrolling `.tar.gz` archives with precompiled binaries and etc.

## Features

* __native-tls__ Use native-tls for HTTPS (by default)
* __rust-tls__ Use rusttls for HTTPS

## Usage example

```rust
use fetch_unroll::fetch_unroll;

let pack_url = format!(
    "{base}/{user}/{repo}/releases/download/{ver}/{pkg}_{prof}.tar.gz",
    base = "https://github.com",
    user = "katyo",
    repo = "oboe-rs",
    pkg = "liboboe-ext",
    ver = "0.1.0",
    prof = "release",
);

let dest_dir = "target/test_download";

// Creating destination directory
std::fs::create_dir_all(dest_dir).unwrap();

// Fetching and unrolling archive
fetch_unroll(pack_url, dest_dir, Config::default()).unwrap();
```
