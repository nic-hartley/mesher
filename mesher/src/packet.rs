use crate::prelude::*;

#[derive(Debug)]
pub(crate) enum Chunk {
  Message(Vec<u8>),
  Transport(String),
  // Reply(...),
  Encrypted(Vec<u8>),
}

impl Chunk {
  fn encrypt(self, key: PublicKey) -> Vec<u8> {
    let mut b = vec![];
    let raw = match self {
      Chunk::Message(mut m) => {
        b.push(0u8);
        b.append(&mut m);
        b
      }
      Chunk::Transport(t) => {
        b.push(1u8);
        b.append(&mut t.into_bytes());
        b
      }
      Chunk::Encrypted(v) => return v,
    };
    key.encrypt(&raw)
  }

  fn decrypt_onekey(bytes: &[u8], key: &SecretKey) -> Result<Chunk, ()> {
    let mut attempt_dec = key.decrypt(bytes)?;
    if attempt_dec.is_empty() {
      return Err(());
    }
    match attempt_dec[0] {
      0 => Ok(Chunk::Message(attempt_dec.drain(1..).collect())),
      1 => Ok(Chunk::Transport(
        String::from_utf8(attempt_dec.drain(1..).collect()).map_err(|_| ())?,
      )),
      _ => Err(()),
    }
  }

  fn decrypt(bytes: Vec<u8>, keys: &[SecretKey]) -> Chunk {
    for key in keys {
      if let Ok(dec) = Self::decrypt_onekey(&bytes, key) {
        return dec;
      }
    }
    Chunk::Encrypted(bytes)
  }
}

#[derive(Default)]
pub struct Packet {
  chunks: Vec<(Chunk, PublicKey)>,
}

impl Packet {
  pub fn add_message(mut self, data: &[u8], target_pkey: &PublicKey) -> Packet {
    self.chunks.push((Chunk::Message(data.to_vec()), target_pkey.clone()));
    self
  }

  pub fn add_hop(mut self, path: String, node_pkey: &PublicKey) -> Packet {
    self.chunks.push((Chunk::Transport(path), node_pkey.clone()));
    self
  }

  pub(crate) fn into_bytes(self) -> Result<Vec<u8>, MesherFail> {
    let packet = self.chunks.into_iter().map(|(c, k)| c.encrypt(k)).collect::<Vec<_>>();
    bincode::serialize(&packet).map_err(|e| MesherFail::Other(Box::new(e)))
  }

  pub(crate) fn from_bytes(packet: &[u8], keys: &[SecretKey]) -> Result<Vec<Chunk>, MesherFail> {
    bincode::deserialize::<Vec<Vec<u8>>>(packet)
      .map(|packet| packet.into_iter().map(|c| Chunk::decrypt(c, keys)).collect())
      .map_err(|_| MesherFail::InvalidPacket)
  }
}
