#![feature(once_cell)]

#[macro_use]
extern crate clone_macro;

mod bindings;

use std::{
    cell::{OnceCell, RefCell, UnsafeCell},
    fmt,
    ops::{self, Deref},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

pub struct Readable<T> {
    value: Rc<UnsafeCell<T>>,
    store: bindings::Readable,
    set_store: Rc<OnceCell<js_sys::Function>>,
    _set_store_closure: Closure<dyn FnMut(js_sys::Function)>,
    #[allow(clippy::type_complexity)]
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
    fn init_store<F>(initial_value: Rc<UnsafeCell<T>>, mapping_fn: F) -> Self
    where
        F: FnMut(&T) -> JsValue + 'static,
    {
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
    }

    /// Creates a `Readable` Svelte store.
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
    pub fn new_mapped<F>(initial_value: T, mapping_fn: F) -> Self
    where
        F: FnMut(&T) -> JsValue + 'static,
    {
        Self::init_store(Rc::new(UnsafeCell::new(initial_value)), mapping_fn)
    }

    pub fn set(&mut self, new_value: T) {
        // SAFETY:
        // This is safe because this function is the only way to
        // mutate T, which already requires an &mut Self, so the
        // borrow checker will make sure no outstanding aliases
        // are possible
        let value = unsafe { &mut *self.value.get() };

        *value = new_value;

        if let Some(set_fn) = self.set_store.get() {
            set_fn
                .call1(&JsValue::NULL, &self.mapped_set_fn.borrow_mut()(value))
                .expect("failed to set readable store");
        }
    }

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

        let o = f(value);

        if let Some(set_fn) = self.set_store.get() {
            set_fn
                .call1(&JsValue::NULL, &self.mapped_set_fn.borrow_mut()(value))
                .expect("failed to set readable store");
        }

        o
    }

    pub fn get_store(&self) -> JsValue {
        self.store.clone()
    }
}
