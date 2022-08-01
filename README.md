# svelte-store

## Svelte Store Bindings

This crate is intended to make working with svelte stores
easy and ergonomic. Specifically, the goal is to allow
easier use of Rust as the backend of a svelte app where
the UI can directly react to changes that happen with
the Rust-WASM world.

This crate exposes one struct, mainly [`Readable`], which
allows seemless management of readable Svelte stores in JS.
Despite it's name, [`Readable`] can be written to from Rust,
but only yields a `Readable` store to JS, making sure that
mutation can only happen within Rust's safety guarantees.

These stores can additionally be annotated with Typescript types
to provide better safety from the JS side. To see how, check out
the [`Readable::get_store`] example. (Note: [`Readable::get_store`]
fn and example is only available on `wasm32` targets)

License: MIT

