//! # Svelte Store Bindings
//!
//! This crate is intended to make working with svelte stores
//! easy and ergonomic. Specifically, the goal is to allow
//! easier use of Rust as the backend of a svelte app where
//! the UI can directly react to changes that happen with
//! the Rust-WASM world.
//!
//! This crate exposes one struct, mainly [`Readable`], which
//! allows seemless management of readable Svelte stores in JS.
//! Despite it's name, [`Readable`] can be written to from Rust,
//! but only yields a `Readable` store to JS, making sure that
//! mutation can only happen within Rust's safety guarantees.
//!
//! These stores can additionally be annotated with Typescript types
//! to provide better safety from the JS side. To see how, check out
//! the [`Readable::get_store`] example. (Note: [`Readable::get_store`]
//! fn and example is only available on `wasm32` targets)

#![feature(once_cell)]

#[cfg(target_arch = "wasm32")]
#[macro_use]
extern crate clone_macro;

#[cfg(target_arch = "wasm32")]
mod bindings;

#[cfg(target_arch = "wasm32")]
use std::cell::{OnceCell, RefCell};
use std::{
    cell::UnsafeCell,
    fmt,
    ops::{self, Deref},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

/// Rust-managed `Readable` Svelte store.
pub struct Readable<T> {
    value: Rc<UnsafeCell<T>>,
    #[cfg(target_arch = "wasm32")]
    store: bindings::Readable,
    #[cfg(target_arch = "wasm32")]
    set_store: Rc<OnceCell<js_sys::Function>>,
    #[cfg(target_arch = "wasm32")]
    _set_store_closure: Closure<dyn FnMut(js_sys::Function)>,
    #[allow(clippy::type_complexity)]
    #[cfg(target_arch = "wasm32")]
    mapped_set_fn: Rc<RefCell<dyn FnMut(&T) -> JsValue>>,
}

impl<T> fmt::Debug for Readable<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Readable").field(self.deref()).finish()
    }
}

impl<T> fmt::Display for Readable<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T> Default for Readable<T>
where
    T: Default + Clone + Into<JsValue> + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// [`Readable`] relies on the fact that only one instance
/// can exist at a time to provide transparent dereferencing
/// to the inner value. As a result, it is unsound to implement
/// [`Clone`]. If you need shared mutability, try using
/// [`Rc`](std::rc::Rc) and [`RefCell`](std::cell::RefCell).
impl<T> ops::Deref for Readable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // This is safe because the `set` fn is the only way to
        // mutate T, which already requires an &mut Self, so the
        // borrow checker will make sure no outstanding aliases
        // are possible
        unsafe { &*self.value.get() }
    }
}

impl<T: 'static> Readable<T> {
    #[allow(unused_variables)]
    fn init_store<F>(initial_value: Rc<UnsafeCell<T>>, mapping_fn: F) -> Self
    where
        F: FnMut(&T) -> JsValue + 'static,
    {
        #[cfg(target_arch = "wasm32")]
        let this = {
            let mapped_set_fn = Rc::new(RefCell::new(
                Box::new(mapping_fn) as Box<dyn FnMut(&T) -> JsValue>
            ));

            let set_store = Rc::new(OnceCell::new());

            let start = Closure::<dyn FnMut(js_sys::Function)>::new(clone!(
                [set_store, initial_value, mapped_set_fn],
                move |set_fn: js_sys::Function| {
                    // Since the value might have changed from the moment the
                    // store was created, we need to set the store again
                    let _ = set_fn.call1(
                        &JsValue::NULL,
                        &(mapped_set_fn.borrow_mut())(unsafe {
                            &*initial_value.get()
                        }),
                    );

                    let _ = set_store.set(set_fn);
                }
            ));

            let store = bindings::readable(
                mapped_set_fn.borrow_mut()(unsafe { &*initial_value.get() }),
                &start,
            );

            Self {
                value: initial_value,
                store,
                set_store,
                _set_store_closure: start,
                mapped_set_fn,
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        let this = {
            Self {
                value: initial_value,
            }
        };

        this
    }

    /// Creates a `Readable` Svelte store.
    ///
    /// This function is only implemented for types that can be converted
    /// into [`JsValue`]. This includes all types annotated with
    /// `#[wasm_bindgen]`. If your type does not provide an [`Into<JsValue>`]
    /// implementation, use [`Readable::new_mapped`] instead.
    ///
    /// # Examples
    ///
    /// Using a type that already provides an implementation of
    /// [`Into<JsValue>`].
    ///
    /// ```
    /// use svelte_store::Readable;
    ///
    /// let store = Readable::new(0u8);
    /// ```
    ///
    /// Using a type annotated with `#[wasm_bindgen]`.
    ///
    /// ```
    /// use svelte_store::Readable;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[derive(Clone)]
    /// #[wasm_bindgen]
    /// pub struct MyStruct;
    ///
    /// let store = Readable::new(MyStruct);
    /// ```
    pub fn new(initial_value: T) -> Self
    where
        T: Clone + Into<JsValue>,
    {
        Self::init_store(Rc::new(UnsafeCell::new(initial_value)), |v| {
            v.clone().into()
        })
    }

    /// Creates a new `Readable` Svelte store which calls its mapping fn each
    /// time the store is set, to produce a [`JsValue`].
    ///
    /// This method should be used whenever [`Readable::new`] cannot be,
    /// due to lacking trait compatibility.
    ///
    /// # Examples
    ///
    /// Creating a store of [`Vec<u8>`].
    ///
    /// ```
    /// use svelte_store::Readable;
    /// use wasm_bindgen::prelude::*;
    ///
    /// let values = vec![7u8; 7];
    ///
    /// let store = Readable::new_mapped(values, |values: &Vec<u8>| {
    ///     values
    ///         .iter()
    ///         .cloned()
    ///         .map(JsValue::from)
    ///         .collect::<js_sys::Array>()
    ///         .into()
    /// });
    /// ```
    pub fn new_mapped<F>(initial_value: T, mapping_fn: F) -> Self
    where
        F: FnMut(&T) -> JsValue + 'static,
    {
        Self::init_store(Rc::new(UnsafeCell::new(initial_value)), mapping_fn)
    }

    /// Sets the value of the store, notifying all store
    /// listeners the value has changed.
    pub fn set(&mut self, new_value: T) {
        // SAFETY:
        // This is safe because this function is the only way to
        // mutate T, which already requires an &mut Self, so the
        // borrow checker will make sure no outstanding aliases
        // are possible
        let value = unsafe { &mut *self.value.get() };

        *value = new_value;

        #[cfg(target_arch = "wasm32")]
        if let Some(set_fn) = self.set_store.get() {
            set_fn
                .call1(&JsValue::NULL, &self.mapped_set_fn.borrow_mut()(value))
                .expect("failed to set readable store");
        }
    }

    /// Calls the provided `f` with a `&mut T`, returning
    /// whatever `f` returns. After this function is called,
    /// the store will be updated and all store listeners will
    /// be notified.
    pub fn set_with<F, O>(&mut self, f: F) -> O
    where
        F: FnOnce(&mut T) -> O,
    {
        // SAFETY:
        // This is safe because this function is the only way to
        // mutate T, which already requires an &mut Self, so the
        // borrow checker will make sure no outstanding aliases
        // are possible
        let value = unsafe { &mut *self.value.get() };

        #[allow(clippy::let_and_return)]
        let o = f(value);

        #[cfg(target_arch = "wasm32")]
        if let Some(set_fn) = self.set_store.get() {
            set_fn
                .call1(&JsValue::NULL, &self.mapped_set_fn.borrow_mut()(value))
                .expect("failed to set readable store");
        }

        o
    }

    /// Gets the store that can be directly passed to JS and subscribed
    /// to.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wasm_bindgen::prelude::*;
    /// use svelte_store::Readable;
    ///
    /// #[wasm_bindgen(typescript_custom_section)]
    /// const TYPESCRIPT_TYPES: &str = r#"
    /// import type { Readable } from "svelte/store";
    ///
    /// type ReadableNumber = Readable<number>;
    /// "#;
    ///
    /// #[wasm_bindgen]
    /// extern "C" {
    ///     #[wasm_bindgen(typescript_type = "ReadableNumber")]
    ///     type ReadableNumber;
    /// }
    ///
    /// #[wasm_bindgen]
    /// pub struct MyStruct {
    ///     my_number: Readable<u8>,
    /// }
    ///
    /// #[wasm_bindgen]
    /// impl MyStruct {
    ///     #[wasm_bindgen(getter)]
    ///     pub fn number(&self) -> ReadableNumber {
    ///         self.my_number.get_store().into()
    ///     }
    /// }
    /// ```
    pub fn get_store(&self) -> JsValue {
        #[cfg(not(target_arch = "wasm32"))]
        panic!(
            "`Readable::get_store()` can only be called \
             within `wasm32` targets"
        );

        #[cfg(target_arch = "wasm32")]
        return self.store.clone();
    }
}
