use js_sys::Array;
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

#[macro_export]
macro_rules! create_array_type {
    ( name: $name:ident , full_js_type: $full_js_type:literal, rust_inner_type: $rust_inner_type:ident ) => {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
            #[::wasm_bindgen::prelude::wasm_bindgen(typescript_type = $full_js_type )]
            pub type $name;
        }
        impl From<Vec<$rust_inner_type>> for $name {
            fn from(vec: Vec<$rust_inner_type>) -> Self {
                use wasm_bindgen::JsCast;
                vec.into_iter()
                    .map(::wasm_bindgen::JsValue::from)
                    .collect::<::js_sys::Array>()
                    .unchecked_into()
            }
        }
    };
}

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
            use ::js_sys::{Object, Reflect};
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
