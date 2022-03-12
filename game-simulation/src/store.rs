use std::{cell::RefCell, collections::HashMap, rc::Rc};

use js_sys::{Array, Function};
use wasm_bindgen::{
    prelude::{wasm_bindgen, Closure},
    JsCast, JsValue,
};

#[wasm_bindgen]
pub struct Writable {
    value: JsValue,
    subscribers: Rc<RefCell<HashMap<u64, Function>>>,
    i: u64,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_SECTION_UNSUBSCRIBE_FUNCTION: &'static str =
    "export type UnsubscribeFunction = () => void;";
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "UnsubscribeFunction")]
    pub type UnsubscribeFunction;
}

#[wasm_bindgen]
impl Writable {
    pub fn subscribe(&mut self, subscriber: Function) -> Result<UnsubscribeFunction, JsValue> {
        subscriber.call1(&JsValue::UNDEFINED, &self.value)?;
        let i = self.i;
        self.i += 1;

        self.subscribers.borrow_mut().insert(i, subscriber);

        let weak_ref = Rc::downgrade(&self.subscribers);

        Ok(Closure::once_into_js(Box::new(move || {
            if let Some(subs) = weak_ref.upgrade() {
                subs.borrow_mut().remove(&i);
            }
        }))
        .unchecked_into())
    }
    pub fn set(&mut self, val: JsValue) -> Array {
        self.value = val;
        let errors = Array::new();
        for sub in self.subscribers.borrow_mut().values() {
            if let Err(e) = sub.call1(&JsValue::UNDEFINED, &self.value) {
                errors.push(&e);
            }
        }
        errors
    }

    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(value: JsValue) -> Self {
        Self {
            value,
            subscribers: Default::default(),
            i: 0,
        }
    }
}
