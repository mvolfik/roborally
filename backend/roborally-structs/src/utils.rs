#[macro_export]
macro_rules! create_array_type {
    ( name: $name:ident , full_js_type: $full_js_type:literal, rust_inner_type: $rust_inner_type:ident ) => {
        #[::wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
            #[::wasm_bindgen::prelude::wasm_bindgen(typescript_type = $full_js_type )]
            pub type $name;
        }
        impl FromIterator<$rust_inner_type> for $name {
            fn from_iter<T: IntoIterator<Item = $rust_inner_type>>(iter: T) -> Self {
                use ::wasm_bindgen::JsCast;
                iter.into_iter()
                    .map(::wasm_bindgen::JsValue::from)
                    .collect::<::js_sys::Array>()
                    .unchecked_into()
            }
        }
    };
}
