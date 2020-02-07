# Simple fetch and unroll .tar.gz archives

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache--2.0-brightgreen.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io Package](https://img.shields.io/crates/v/fetch_unroll.svg?style=popout)](https://crates.io/crates/fetch_unroll)
[![Docs.rs API Docs](https://docs.rs/fetch_unroll/badge.svg)](https://docs.rs/fetch_unroll)
[![Travis-CI Status](https://travis-ci.com/katyo/fetch_unroll.svg?branch=master)](https://travis-ci.com/katyo/fetch_unroll)

Simple functions intended to use in __Rust__ `build.rs` scripts for tasks which related to fetching from _HTTP_ and unrolling `.tar.gz` archives with precompiled binaries and etc.

## Usage example

```rust
use fetch_unroll::Fetch;

let pack_url = format!(
    concat!("{base}/{user}/{repo}/releases/download/",
            "{package}-{version}/{package}_{target}_{profile}.tar.gz"),
    base = "https://github.com",
    user = "katyo",
    repo = "aubio-rs",
    package = "libaubio",
    version = "0.5.0-alpha",
    target = "armv7-linux-androideabi",
    profile = "debug",
);

let dest_dir = "target/test_download";

// Fetching and unrolling archive
Fetch::from(pack_url)
    .unroll().strip_components(1).to(dest_dir)
    .unwrap();
```
