<div align="center">
  <h1>jsurl</h1>
  <p>
    <strong>A human-readable alternative to JSON + URL encoding, especially for use in URL query parameters.</strong>
  </p>
  <p>

[![crates.io][crates.io shield]][crates.io link]
[![Documentation][docs.rs badge]][docs.rs link]
![Rust CI][github ci badge]
[![rustc 1.0+]][Rust 1.0]
<br />
<br />
[![Dependency Status][deps.rs status]][deps.rs link]
[![Download Status][shields.io download count]][crates.io link]

  </p>
</div>

[crates.io shield]: https://img.shields.io/crates/v/jsurl?label=latest
[crates.io link]: https://crates.io/crates/jsurl
[docs.rs badge]: https://docs.rs/jsurl/badge.svg?version=0.1.0
[docs.rs link]: https://docs.rs/jsurl/0.1.0/jsurl/
[github ci badge]: https://github.com/Chriscbr/jsurl/actions/workflows/rust.yml/badge.svg
[rustc 1.0+]: https://img.shields.io/badge/rustc-1.0%2B-blue.svg
[Rust 1.0]: https://blog.rust-lang.org/2015/05/15/Rust-1.0.html
[deps.rs status]: https://deps.rs/repo/github/Chriscbr/jsurl/status.svg
[deps.rs link]: https://deps.rs/crate/jsurl/0.1.0
[shields.io download count]: https://img.shields.io/crates/d/jsurl.svg

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
jsurl = "0.1"
```

### Description

<!-- cargo-rdme start -->

This crate is a Rust implementation of the [jsurl](https://github.com/Sage/jsurl)
serialization format. It is a more compact and human-readable alternative to plain URL encoding
for including JSON in URLs.

#### Example

```rust
use jsurl::{deserialize, serialize};
use serde_json::json;

let obj = json!({
    "name": "John Doe",
    "age": 42,
    "children": ["Mary", "Bill"]
});

let serialized = serialize(&obj);
assert_eq!(serialized, "~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))");

let deserialized = deserialize("~(name~'John*20Doe~age~42~children~(~'Mary~'Bill))").unwrap();
assert_eq!(deserialized, obj);
```

<!-- cargo-rdme end -->

## License

Dual-licensed for compatibility with the Rust project.

Licensed under the Apache License Version 2.0: http://www.apache.org/licenses/LICENSE-2.0,
or the MIT license: http://opensource.org/licenses/MIT, at your option.
