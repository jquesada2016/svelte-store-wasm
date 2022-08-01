use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "svelte/store")]
extern "C" {
    pub type Readable;
    // pub type Writable;

    pub fn readable(
        initial_value: JsValue,
        start: &Closure<dyn FnMut(js_sys::Function)>,
    ) -> Readable;

    // pub fn writable(initial_value: JsValue) -> Writable;

    // #[wasm_bindgen(method)]
    // pub fn set(this: &Writable, value: JsValue);

    // #[wasm_bindgen(method)]
    // pub fn update(this: &Writable, f: &mut dyn FnMut(JsValue) -> JsValue);

    // #[wasm_bindgen(method)]
    // pub fn subscribe(
    //     this: &Writable,
    //     f: &Closure<dyn FnMut(JsValue)>,
    // ) -> js_sys::Function;
}
