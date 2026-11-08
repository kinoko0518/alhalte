use aes_gcm_siv::{Aes256GcmSiv, Key};

pub mod message;
pub mod tin;

pub type RoomKey = Key<Aes256GcmSiv>;
