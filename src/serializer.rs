use crate::protocol::{decode,encode,Frame};
use crate::utils::encrypt::Encryptor;

pub struct Serializer {
    encryptor: Encryptor,
}

impl Serializer {
    pub fn new(method: &str, password: &str) -> Result<Self, String> {
        let encryptor = Encryptor::new(method, password);
        Ok(Self {encryptor  })
    }

   pub fn serialize(&self, frame: &Frame) -> Vec<u8> {
        let mut  data_bytes = encode(frame);
        self.encryptor.encrypt(&mut data_bytes);
        data_bytes[..].to_vec()
    }

    pub fn deserialize(&self, data: &[u8]) -> Result<Frame, String> {
        let mut data2 = data.to_vec();
        let ret = self.encryptor.decrypt(&mut data2);
        decode(&data2).map_err(|err| format!("Error during decoding: {}", err))
    }

    // pub fn serialize(&self, frame: &Frame) -> Vec<u8> {
    //     let data_bytes = encode(frame);
    //     return data_bytes;
    //     // self.encryptor.encrypt(&data_bytes[..])
    // }

    // pub fn deserialize(&self, data: &[u8]) -> Result<Frame, String> {
    //     // let buf = self.encryptor.decrypt(&data[..]).map_err(|err| format!("Error during decryption: {}", err))?;
    //     decode(data).map_err(|err| format!("Error during decoding: {}", err))
    // }
}
