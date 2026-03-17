# annotate-rs

[![crates.io](https://img.shields.io/crates/v/annotate.svg)](https://crates.io/crates/annotate)

`annotate` is a Rust annotation and reflection framework.

- mark functions and modules with `#[pragma(...)]`
- create custom annotations
- attach structured metadata such as strings, booleans, integers, and Rust types
- generate a static annotation environment during `build.rs`
- query annotated items at runtime with predicates over modules and functions
- call discovered functions when their signature is known

The workspace is split into:

- `annotate`: runtime API and public macro re-exports
- `annotate-build`: build-time scanner and code generator
- `annotate-derive`: procedural macro implementation

## Installation

Add `annotate` to your crate dependencies and `annotate-build` to your build dependencies:

```toml
[dependencies]
annotate = "x.y.z"

[build-dependencies]
annotate-build = "x.y.z"
```

## Quick Start

Create a `build.rs` file that scans the crate and generates the annotation environment:

```rust
fn main() {
    annotate_build::build();
}
```

Annotate functions or modules in your crate and include the generated environment:

```rust
annotate::environment!();

#[pragma(tag = "math", active = true)]
pub mod operations {
    use annotate::*;

    #[pragma(kind = "sum")]
    fn add(a: i32, b: i32) -> i32 {
        a + b
    }
}
```

Query the functions at runtime:

```rust
use annotate::Value;

fn main() {
    let function = environment()
        .find_functions_such_that(&|function| {
            function.has_attribute_such_that(&|attribute| {
                attribute.name() == "kind" && attribute.is_str("sum")
            })
        })
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(function.name(), "add");

    let value = function.call::<fn(i32, i32) -> i32, _>(|f| f(2, 3));
    assert_eq!(value, 5);
}
```

Querying modules by associated type:

```rust
annotate::environment!();

#[pragma(associated_type = String)]
mod module_with_associated_type {
    
}

fn main() {
    let module = environment()
        .find_modules_such_that(&|module| {
            module.has_attribute_such_that(&|attribute| {
                attribute.name() == "associated_type" && attribute.is_type::<String>()
            })
        })
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(module.name(), "module_with_associated_type");
}
```

## What Gets Generated

At build time, `annotate-build` scans the crate source and generates a static `Environment` containing:

- annotated functions
- annotated modules
- their paths
- their attributes
- links between modules and functions

The generated Rust code is written into `OUT_DIR` and included through `annotate::environment!()`.

## Limitations

- `#[pragma]` currently supports functions and modules only.
- Calling discovered functions still requires the caller to know the exact function signature.
- Dynamic return support is limited and primarily intended for zero-argument functions.
- Build-time code generation is required; this crate is not purely macro-only.

## License

Copyright feenk gmbh.

Licensed under MIT. See [LICENSE](LICENSE).
