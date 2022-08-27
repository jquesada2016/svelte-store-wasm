use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "svelte/store")]
extern "C" {
    pub type Readable;

    #[wasm_bindgen(extends = Readable)]
    pub type Writable;

    pub fn writable(initial_value: JsValue) -> Writable;

    pub fn derived(
        store: &Readable,
        map_fn: &Closure<dyn FnMut(JsValue) -> JsValue>,
    ) -> Readable;

    #[wasm_bindgen(method)]
    pub fn set(this: &Writable, value: JsValue);

    #[wasm_bindgen(method)]
    pub fn update(this: &Writable, f: &mut dyn FnMut(JsValue) -> JsValue);

    #[wasm_bindgen(method)]
    pub fn subscribe(
        this: &Readable,
        f: &Closure<dyn FnMut(JsValue)>,
    ) -> js_sys::Function;
}
