# Saving and loading

Saving and loading the state of a story can be done using the serialization and 
deserialization methods of [`serde`](https://serde.rs). 

If the `serde_support` feature is enabled (see [Set-up][serde_support]
for more information), `inkling` derives the required method for all of its objects. 
It is then possible to use any compatible serializer and deserializer to save the 
state into some object on disk, and restore the data from that object. 

Some supported data formats are listed on [this page](https://serde.rs/#data-formats).


## Example: using JSON

The [`serde_json`](https://github.com/serde-rs/json) crate uses JSON text files as storage. 
In `Cargo.toml`, add 

```toml
serde_json = 1.0
```

to your dependencies and ensure that the [`serde_support`][serde_support] feature 
is enabled for `inkling`.

### Converting from `Story` to `String`

```rust,ignore
use serde_json;

let serialized_story: String = serde_json::to_string(&story).unwrap();

// write `serialized_story` to a text file
```

### Restoring a `Story` from `String`

```rust,ignore
use serde_json;

// `serialized_story` is read from a text file

let story: Story = serde_json::from_str(&serialized_story).unwrap();
```

[serde_support]: set-up.md#adding-serde-support