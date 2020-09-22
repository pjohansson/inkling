# Set-up

To use `inkling` in your Rust project, add this line to your dependencies in `Cargo.toml`:

```toml
[dependencies]
#
inkling = "0.12.6"
```

By default, `inkling` has no additional dependencies. Extra features which carry dependencies
can be opted in to.

## Adding `serde` support

The [`serde`](https://serde.rs/) library is widely used to serialize and deserialize data, which can be used 
to [save and restore](./saving-and-loading.md) an object. Support for this can be added to `inkling` by enabling the `serde_support` 
feature. This adds `serde` as a dependency.

```toml
[dependencies]
#
inkling = { version = "0.12.6", features = ["serde_support"] }
```


## Randomization support

The `Ink` language supports a few randomized features like [shuffle sequences](../features/sequences.md#shuffle-sequences).
These are optional and can be enabled with the `random` feature. This adds 
a dependency to `rand` and its sub project `rand_chacha`.

```toml
[dependencies]
#
inkling = { version = "0.12.6", features = ["random"] }
```

If this feature is not enabled, shuffle sequences will behave as cycle sequences.