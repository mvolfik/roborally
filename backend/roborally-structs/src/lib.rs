#![warn(clippy::nursery)]
#![allow(clippy::use_self)]
#![allow(clippy::missing_const_for_fn)]
#![warn(clippy::pedantic)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::unused_unit)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::many_single_char_names)]
// restrictions
#![warn(clippy::allow_attributes_without_reason)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::else_if_without_else)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::let_underscore_must_use)]
#![warn(clippy::shadow_reuse)]
#![warn(clippy::shadow_same)]
#![warn(clippy::shadow_unrelated)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_add)]
#![warn(clippy::string_to_string)]
#![warn(clippy::try_err)]
// features
#![feature(concat_idents)]
#![feature(const_precise_live_drops)]
#![feature(iter_intersperse)]
#![feature(pattern)]
#![feature(thread_id_value)]

pub mod animations;
pub mod card;
pub mod game_map;
pub mod game_state;
pub mod logging;
pub mod position;
pub mod tile;
pub mod tile_type;
pub mod transform;
pub mod transport;
pub mod utils;

#[cfg(feature = "client")]
pub use wasm_bindgen;
