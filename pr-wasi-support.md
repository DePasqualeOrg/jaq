## Summary

This PR adds `wasm32-wasi` platform support by making `rustyline` an optional dependency behind a `repl` feature flag. The REPL is not available on WASI, but all other jaq functionality works.

## Changes

- `jaq/Cargo.toml`: Add `repl` feature (enabled by default) gating the `rustyline` dependency. Building with `--no-default-features` excludes rustyline, which doesn't compile on WASI due to `fd-lock` lacking platform support.
- `jaq/src/funs.rs`: Gate rustyline imports and `repl_with` function with `#[cfg(not(target_os = "wasi"))]`. The `repl` filter prints an error message on WASI instead of panicking.

## Motivation

Enables jaq to compile and run on `wasm32-wasip1` for use in WASI-compatible runtimes. All JSON query functionality works – only the interactive REPL is unavailable.

## Build

```
cargo build --target wasm32-wasip1 --release --no-default-features
```

Produces a 3.0 MB binary. Tested with filter expressions, sort, select, raw output, to_entries/from_entries.
