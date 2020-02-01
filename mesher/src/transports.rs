// TODO: don't use String error messages, pass relevant data

#[non_exhaustive]
#[derive(Debug)]
pub enum TransportFail {
  // the packet we received isn't formatted validly
  InvalidPacket,

  // the URL is syntactically invalid
  InvalidURL(String),
  // the scheme hasn't been registered with the Mesher
  UnregisteredScheme(String),

  // could not set up to listen on the given scheme
  SetupFailure(String),

  // could not send data along the given path
  SendFailure(String),

  // could not start listening along the given path
  ListenFailure(String),

  // could not pull data down from listened paths
  ReceiveFailure(String),

  // an arbitary other error
  Other(Box<dyn std::error::Error>),
}

pub trait Transport {
  fn new(scheme: &str) -> Result<Self, TransportFail>
  where
    Self: Sized;
  fn send(&mut self, path: String, blob: Vec<u8>) -> Result<(), TransportFail>;
  fn listen(&mut self, path: String) -> Result<(), TransportFail>;
  fn receive(&mut self) -> Result<Vec<Vec<u8>>, TransportFail>;
}
