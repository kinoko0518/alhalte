use std::string::FromUtf8Error;

use aes_gcm_siv::{
    Aes256GcmSiv, KeyInit, Nonce,
    aead::{Aead, Generate},
};

use crate::RoomKey;

pub struct EncryptedMessage(Vec<u8>);

pub enum MessageError {
    EncryptionFailed(aes_gcm_siv::Error),
    NonceNotFound,
    CiphertextNotFound,
    InvalidAsUtf8(FromUtf8Error),
}
const NONCE_WIDTH: usize = size_of::<Nonce>();

impl EncryptedMessage {
    pub fn encrypt(message: String, room_key: impl Into<RoomKey>) -> Result<Self, MessageError> {
        let nonce = Nonce::generate();
        let padded = {
            let message = message.as_bytes();
            let mut padded = nonce.to_vec();
            padded.extend(message);
            let target_len = padme_padding::Padme::padding_len(padded.len());
            padded.extend(vec![0; target_len]);
            padded
        };
        let cipher = Aes256GcmSiv::new(&room_key.into());

        let ciphertext = cipher
            .encrypt(&nonce, &padded[..])
            .map_err(|e| MessageError::EncryptionFailed(e))?;

        Ok(Self(ciphertext))
    }

    fn nonce(&self) -> Option<&[u8; NONCE_WIDTH]> {
        self.0.first_chunk::<NONCE_WIDTH>()
    }

    fn ciphertext(&self) -> Option<&[u8]> {
        self.0.get(NONCE_WIDTH..)
    }

    pub fn decrypt(&self, room_key: impl Into<RoomKey>) -> Result<String, MessageError> {
        let cipher = Aes256GcmSiv::new(&room_key.into());

        let nonce = self.nonce().ok_or(MessageError::NonceNotFound)?.into();
        let ciphertext = self.ciphertext().ok_or(MessageError::CiphertextNotFound)?;

        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(MessageError::EncryptionFailed)?;

        String::from_utf8(decrypted).map_err(MessageError::InvalidAsUtf8)
    }
}
