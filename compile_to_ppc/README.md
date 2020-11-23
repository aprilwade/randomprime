This directory contains crates that are compiled to PPC and some associated helper proc-macros. Compiling these crates requires having  the `powerpc-unknown-linux-gnu` target installed. You can install them with `rustup`:

```
rustup target add --toolchain stable powerpc-unknown-linux-gnu
```

A nightly-only feature is required to build `primeapi-rs`, so the `RUSTC_BOOTSTRAP=1` environment variable must be set when building most anything in this directory. Thus, the recommended command to compile any of these crates is:

```
RUSTC_BOOTSTRAP=1 cargo rustc -p rel_patches --release --target powerpc-unknown-linux-gnu -- -C relocation-model=static
```
