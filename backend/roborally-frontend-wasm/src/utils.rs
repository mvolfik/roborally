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
