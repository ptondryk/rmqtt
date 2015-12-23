use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::str;

use mqtt::*;
mod mqtt;

trait HandlesMessage {
    fn handle_message(&self, topic: &str, message: &str);
}

struct Mqtt {
    host: String,
    user: String,
    password: String
}

impl Mqtt {
    fn new(host: &str, user: &str, password: &str) -> Mqtt {
        Mqtt {
            host: host.to_string(),
            user: user.to_string(),
            password: password.to_string()
        }
    }

    fn connect(&mut self) {
        // connect to the broker-server
        let mut stream = TcpStream::connect(&*self.host).unwrap();

        // send CONNECT packet to mqtt-broker
        Mqtt::send(&mut stream, &CONNECT::new_with_authentication("testClientId", "system", "manager").as_bytes().into_boxed_slice());

        // start receive thread
        thread::spawn(move || {
            Mqtt::receive(&mut stream);
        });
    }

    fn publish(&mut self, topic: &str, text: &str) -> bool {
        unimplemented!()
    }

    fn subscribe<T: HandlesMessage>(&self, topic: &str, handler: T) -> bool {
        unimplemented!()
    }

    fn send(stream: &mut TcpStream, bytes: &[u8]) -> bool {
        let _ = stream.write(bytes);
        true
    }

    fn receive(stream: &mut TcpStream) {
        println!("start receive thread");

        let mut buffer: Vec<u8> = Vec::new();
        loop {
            for byte in stream.bytes() {
                buffer.push(byte.unwrap());
                let received_packet = mqtt::parse(&buffer);
                match received_packet {
                    Some(ref packet) => {
                        println!("parsed packet");
                        println!("{:?}", packet);
                        buffer.clear();
                    },
                    None => {}
                }
            }
        }
    }
}

fn main() {
    let mut m = Mqtt::new("localhost:1883", "host", "password");
    m.connect();
    loop {}
}
