use std::io::prelude::*;
use std::net::TcpStream;
use mqtt::CtrlPacket;

use std::process::Command;
use std::str;

use std::time::Duration;
use std::thread::sleep;

mod mqtt;

trait HandlesMessage {
    fn handleMessage(&self, topic: &str, message: &str);
}

struct Mqtt {
    host: String,
    stream: Option<TcpStream>,
    user: String,
    password: String
}

impl Mqtt {
    fn new(host: &str, user: &str, password: &str) -> Mqtt {
        Mqtt {
            host: host.to_string(),
            stream: None,
            user: user.to_string(),
            password: password.to_string()
        }
    }

    fn connect(&mut self) {
        self.stream = Some(TcpStream::connect(&*self.host).unwrap());
        self.send(&*CtrlPacket::CONNECT.as_bytes().into_boxed_slice());
        self.receive();
        // unimplemented!()
    }

    fn publish(&mut self, topic: &str, text: &str) -> bool {
        unimplemented!()
    }

    fn subscribe<T: HandlesMessage>(&self, topic: &str, handler: T) -> bool {
        unimplemented!()
    }

    fn send(&mut self, bytes: &[u8]) -> bool {
        if let Some(ref mut stream) = self.stream {
            // TODO check result
            let _ = stream.write(bytes);
            true
        } else {
            panic!("Connection not established!");
        }
    }

    fn receive(&mut self) -> Vec<u8> {
        if let Some(ref mut stream) = self.stream {

            sleep(Duration::from_millis(1000));

            let mut buffer: Vec<u8> = Vec::new();
            stream.read_to_end(&mut buffer);

            if(buffer.len() > 0) {
                let mut a = [0; 20];
                for i in 0..buffer.len() {
                    a[i] = buffer[i];
                }
                let s = match str::from_utf8(&a) {
                    Ok(v) => v,
                    Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                };

                println!("result: {}", s);
            } else {
                println!("empty response");
            }

            buffer
        } else {
            panic!("Connection not established!");
        }
    }
}

fn main() {
    let mut m = Mqtt::new("localhost:1883", "host", "password");
    m.connect();
}
