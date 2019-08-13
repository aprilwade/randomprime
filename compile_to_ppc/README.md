This directory contains crates that are compiled to PPC and some associated helper proc-macros. Compiling these crates requires having the nightly compiler and the `powerpc-unknown-linux-gnu` target installed. You can install them with `rustup`:

```
rustup toolchain install nightly
rustup target add --toolchain nightly powerpc-unknown-linux-gnu
```

The recommended command to compile any of these crates is:

```
cargo +nightly rustc --release --target powerpc-unknown-linux-gnu -- -C relocation-model=static
```
