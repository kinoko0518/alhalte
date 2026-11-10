use std::string::FromUtf8Error;

use aes_gcm_siv::{
    Aes256GcmSiv, KeyInit, Nonce,
    aead::{Aead, Generate},
};

use crate::RoomKey;

pub struct EncryptedMessage(Vec<u8>);

#[derive(Debug)]
pub enum MessageError {
    CipherthingFailed(aes_gcm_siv::Error),
    MessageLenNotFound,
    NonceNotFound,
    CiphertextNotFound,
    InvalidAsUtf8(FromUtf8Error),
}

const NONCE_WIDTH: usize = size_of::<Nonce>();

type MessageLen = u32;
const MESSAGE_BYTEWIDTH: usize = size_of::<MessageLen>();

impl EncryptedMessage {
    pub fn encrypt(message: String, room_key: impl Into<RoomKey>) -> Result<Self, MessageError> {
        let nonce = Nonce::generate();
        let message = message.as_bytes();
        let message_len = message.len();

        let padded = {
            let mut padded = (message_len as MessageLen).to_le_bytes().to_vec();
            padded.extend(message);

            let target_len = padme_padding::Padme::padding_len(padded.len());
            padded.extend(vec![0; target_len]);
            padded
        };
        let cipher = Aes256GcmSiv::new(&room_key.into());

        let mut bin = nonce.to_vec();
        bin.extend(
            cipher
                .encrypt(&nonce, &padded[..])
                .map_err(MessageError::CipherthingFailed)?,
        );

        Ok(Self(bin))
    }

    fn message_len(plaintext: &[u8]) -> Option<usize> {
        plaintext
            .first_chunk::<MESSAGE_BYTEWIDTH>()
            .copied()
            .map(|c| MessageLen::from_le_bytes(c) as usize)
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

        let mut decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(MessageError::CipherthingFailed)?;
        let messagelen = Self::message_len(&decrypted).ok_or(MessageError::MessageLenNotFound)?;

        decrypted.drain(..MESSAGE_BYTEWIDTH);
        decrypted.truncate(messagelen);

        String::from_utf8(decrypted).map_err(MessageError::InvalidAsUtf8)
    }
}

#[test]

fn message_encrypt_decrypt_test() {
    let room_key: RoomKey = [0_u8; 32].into();
    let original = "foo bar baz, ほげ、ふが、ぴよ";

    let message = EncryptedMessage::encrypt(original.to_string(), room_key).unwrap();
    let dmessage = message.decrypt(room_key).unwrap();

    assert_eq!(original, dmessage.as_str())
}
