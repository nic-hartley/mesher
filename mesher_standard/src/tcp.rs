use mesher::prelude::*;

use std::{
  net::{IpAddr, SocketAddr, ToSocketAddrs},
  sync::mpsc::{channel, Receiver, Sender, TryRecvError},
  thread::{sleep, Builder, JoinHandle},
  time::Duration,
};

enum Order {
  Quit,
  Tx(IpAddr, Vec<u8>),
  Rx(SocketAddr),
}

// fn tcp_listen(orders: Receiver<Order>, data: Sender<Vec<u8>>) -> Box<dyn FnOnce() -> ()> {
//   Box::new()
// }

pub struct TCP {
  orders: Sender<Order>,
  data: Receiver<Vec<u8>>,
  scheme: String,
  listener_thread: JoinHandle<()>,
}

impl Transport for TCP {
  fn new(scheme: &str) -> Result<Self, TransportFail> {
    let (orders_in, orders_out) = channel();
    let (data_in, data_out) = channel();

    let thread_code = move || {
      loop {
        match orders_out.try_recv() {
          Ok(Order::Quit) => return,
          Ok(Order::Tx(dest, data)) => println!("Would send {:?} to {:?}", dest, data),
          Ok(Order::Rx(on)) => {
            println!("Would listen on {:?}", on);
            if let Err(_) = data_in.send(vec![1, 2, 3]) {
              // means the other channel is disconnected, so this thread should die too
              return;
            }
          }
          Err(TryRecvError::Empty) => sleep(Duration::from_millis(1000)),
          Err(TryRecvError::Disconnected) => return,
        }
      }
    };

    let thread = Builder::new()
      .name(format!("TCP {}: listener", scheme))
      .spawn(thread_code)
      .map_err(|e| {
        TransportFail::SetupFailure(format!(
          "Faield to start TCP {}: listener: {:?}",
          scheme, e
        ))
      })?;
    Ok(TCP {
      scheme: scheme.to_string(),
      orders: orders_in,
      data: data_out,
      listener_thread: thread,
    })
  }

  fn send(&mut self, path: String, blob: Vec<u8>) -> Result<(), TransportFail> {
    let ip = path.parse().map_err(|e| TransportFail::InvalidURL(format!("{:?}", e)))?;
    self.orders.send(Order::Tx(ip, blob)).map_err(|_| TransportFail::SendFailure(format!("Failed to give TCP {}: data to sending thread", self.scheme)))
  }

  fn listen(&mut self, path: String) -> Result<(), TransportFail> {
    let get_path_fail = || TransportFail::InvalidURL(format!("not a valid socket address format: {}", path));
    let sock = path.to_socket_addrs().map_err(|_| get_path_fail())?.next().ok_or(get_path_fail())?;
    self.orders.send(Order::Rx(sock)).map_err(|_| TransportFail::ListenFailure(format!("Failed to give TCP {}: address to listening thread", self.scheme)))
  }

  fn receive(&mut self) -> Result<Vec<Vec<u8>>, TransportFail> {
    let mut received = vec![];
    loop {
      match self.data.try_recv() {
        Ok(d) => received.push(d),
        Err(TryRecvError::Empty) => break,
        Err(TryRecvError::Disconnected) => return Err(TransportFail::ReceiveFailure(format!("TCP {}: listener disconnected (did the thread die?)", self.scheme))),
      }
    }
    Ok(received)
  }
}

impl Drop for TCP {
  fn drop(&mut self) {
    loop {
      match self.orders.send(Order::Quit) {
        Ok(_) => (),     // other side still alive
        Err(_) => break, // other side dead now
      }
      // don't spinlock so we don't burn CPU.
      sleep(Duration::from_millis(50));
    }
  }
}
