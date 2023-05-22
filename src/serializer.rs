use crate::protocol::{decode,encode,Frame};
// use toolbox::Encryptor;

pub struct Serializer {
    // encryptor: Encryptor,
}

impl Serializer {
    pub fn new(method: &str, password: &str) -> Result<Self, String> {
        // let encryptor = Encryptor::new(method, password).map_err(|err| format!("Error during creating encryptor: {}", err))?;
        Ok(Self {  })
    }

    pub fn serialize(&self, frame: &Frame) -> Vec<u8> {
        let data_bytes = encode(frame);
        return data_bytes;
        // self.encryptor.encrypt(&data_bytes[..])
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<Frame, String> {
        // let buf = self.encryptor.decrypt(&data[..]).map_err(|err| format!("Error during decryption: {}", err))?;
        decode(data).map_err(|err| format!("Error during decoding: {}", err))
    }
}
