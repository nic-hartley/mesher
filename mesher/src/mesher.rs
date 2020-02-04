use {
  rand::prelude::*,
  std::collections::HashMap,
  crate::prelude::*,
};

#[derive(Debug)]
pub struct Message {
  contents: Vec<u8>,
}

impl Message {
  pub fn contents(&self) -> &[u8] {
    &self.contents
  }
}

pub struct Mesher {
  transports: HashMap<String, Box<dyn Transport>>,
  own_skeys: Vec<SecretKey>,
  own_pkeys: Vec<PublicKey>,
  rng: rand::rngs::ThreadRng,
}

impl Mesher {
  pub fn signed(own_skeys: Vec<SecretKey>, _source_sigs: Vec<PublicKey>) -> Mesher {
    // TODO: outgoing packet signature setup
    Mesher::unsigned(own_skeys)
  }
  pub fn unsigned(own_skeys: Vec<SecretKey>) -> Mesher {
    Mesher {
      transports: HashMap::new(),
      own_pkeys: own_skeys.iter().map(SecretKey::pkey).collect(),
      own_skeys,
      rng: ThreadRng::default(),
    }
  }

  pub fn add_transport<T: Transport + 'static>(&mut self, scheme: &str) -> Result<(), TransportFail> {
    self.transports.insert(scheme.to_owned(), Box::new(T::new(scheme)?));
    Ok(())
  }

  #[allow(clippy::borrowed_box)]
  fn get_transport_for_path(&mut self, path: &str) -> Result<&mut Box<dyn Transport>, TransportFail> {
    let scheme = path
      .splitn(2, ':')
      .next()
      .ok_or_else(|| TransportFail::InvalidURL("no colon-delimited scheme segment".to_string()))?
      .to_owned();
    self
      .transports
      .get_mut(&scheme)
      .ok_or(TransportFail::UnregisteredScheme(scheme))
  }

  pub fn listen_on(&mut self, path: &str) -> Result<(), TransportFail> {
    self.get_transport_for_path(path)?.listen(path.to_owned())
  }

  fn random_key(&mut self) -> crate::fail::Result<&PublicKey> {
    self
      .own_pkeys
      .choose(&mut self.rng)
      // .map(Clone::clone)
      .ok_or(crate::fail::Fail::NoKeys)
  }

  fn process_packet(&mut self, pkt: Vec<u8>) -> crate::fail::Result<Vec<Message>> {
    let dis = crate::packet::Packet::from_bytes(&pkt, &self.own_skeys)?;
    let mut messages = vec![];
    for piece in dis {
      match piece {
        crate::packet::Chunk::Message(m) => messages.push(Message { contents: m }),
        crate::packet::Chunk::Transport(to) => self.bounce(&pkt, &to)?,
        crate::packet::Chunk::Encrypted(_) => (), /* piece not meant for us */
      }
    }
    Ok(messages)
  }

  pub fn send(&mut self, message: &[u8], route: crate::packet::SimpleRoute) -> crate::fail::Result<()> {
    let assembled = crate::packet::Packet::along_route(message, route, self.random_key()?).into_bytes()?;
    self.process_packet(assembled)?;
    Ok(())
  }

  fn bounce(&mut self, packet: &[u8], path: &str) -> crate::fail::Result<()> {
    let transport = self.get_transport_for_path(path)?;
    transport.send(path.to_owned(), packet.to_vec())?;
    Ok(())
  }

  pub fn recv(&mut self) -> crate::fail::Result<Vec<Message>> {
    // don't focus too much on how I got this...
    let mut packets = vec![];
    for (_, transport) in self.transports.iter_mut() {
      packets.append(&mut transport.receive()?);
    }
    let mut messages = vec![];
    for p in packets {
      messages.append(&mut self.process_packet(p)?);
    }
    Ok(messages)
  }
}