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

#[macro_export]
macro_rules! js_object {
    { $( $key:expr => $value:expr ),* } => {
        {
            let object = Object::new();
            let mut errs = Vec::new();
            $(
                if let Err(e) = Reflect::set(&object, $key, $value) {
                    errs.push(e);
                }
            )*
            if errs.is_empty() {
                Ok(object)
            } else {
                Err(errs)
            }
        }
    };
}
