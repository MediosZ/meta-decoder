# meta decoder

This crate is for decoding metadata rust libraries (static-lib / dynamic-lib).

## Usage

Prepare a library first.

Note that for dynamic libraries, you need to pass `-Cprefer-dynamic` to rustc.

```bash
cargo r -- --path <path of library>
```

## What is this for?

When developing Rust Loader for [Metacall](https://github.com/metacall/core), we need to inspect the members of compiled libraries, so that we can invoke these functions from other languages.

After investigation, I found there's little resources about how to inspect compiled rust libraries.
I have to dive into the rustc source to find what we need.

So I want to provide a easy path to learn how to decode the metadata.
This crate is not production ready, and it's more like a tutorial.