use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::sync::{Mutex, Arc};
use std::str;

use mqtt::*;
mod mqtt;

trait HandlesMessage {
    fn handle_message(&self, topic: &str, message: &str);
}

struct Mqtt {
    host: String,
    user: String,
    password: String,
    packet_id: i16,
    stream: Mutex<Option<TcpStream>>
}

impl Mqtt {
    fn new(host: &str, user: &str, password: &str) -> Mqtt {
        Mqtt {
            host: host.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            packet_id: 0,
            stream: Mutex::new(None)
        }
    }

    fn connect(&mut self) {
        // connect to the broker-server
        let stream = Arc::new(Mutex::new(Some(TcpStream::connect(&*self.host).unwrap())));
        self.create_connection(stream.clone());

        // send CONNECT packet to mqtt-broker
        self.send(&CONNECT::new_with_authentication("testClientId", "system", "manager")
                    .as_bytes().into_boxed_slice());

        // start receive thread
        thread::spawn(move || {
            self.receive();
        });
    }

    fn create_connection(&mut self, stream: Mutex<Option<TcpStream>>) {
        self.stream = stream;
    }

    fn publish(&mut self, topic: &str, text: &str) -> bool {
        unimplemented!()
    }

    fn subscribe<T: HandlesMessage>(&mut self, topic: &str, handler: T) -> bool {
        // send SUBSCRIBE packet to mqtt-broker
        self.send(&SUBSCRIBE::new(topic, 0, self.packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
        true
    }

    fn send(&self, bytes: &[u8]) -> bool {
        match self.stream.lock() {
            Some(ref valid_stream) => {
                let _ = valid_stream.write(bytes);
                true
            },
            None => {
                panic!("Can't subscribe! Connect first!");
                false
            }
        }
    }

    fn receive(&self) {
        println!("start receive thread");

        let mut buffer: Vec<u8> = Vec::new();
        loop {
            match self.stream.lock() {
                Some(ref valid_stream) => {
                    for byte in valid_stream.bytes() {
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
                },
                None => {}
            }
        }
    }
}

struct ExampleHandler {
    example_var: i16
}

impl ExampleHandler {

}

impl HandlesMessage for ExampleHandler {
    fn handle_message(&self, topic: &str, message: &str) {
        println!("topic: {:?}, message: {:?}", topic, message);
    }
}

fn main() {
    let mut m = Mqtt::new("localhost:1883", "host", "password");
    m.connect();
    m.subscribe("testTopic1", ExampleHandler {
        example_var: 1
    });
    loop {}
}
