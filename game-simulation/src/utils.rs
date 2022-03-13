use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Array<String>")]
    pub type StringArray;
}

impl<T: ToString> From<Vec<T>> for StringArray {
    fn from(v: Vec<T>) -> Self {
        v.into_iter()
            .map(|x| x.to_string())
            .map(JsValue::from)
            .collect::<Array>()
            .unchecked_into()
    }
}
