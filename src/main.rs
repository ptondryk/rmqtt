use std::io::prelude::*;
use std::net::TcpStream;

enum CtrlPacket {
    CONNECT,
    CONNACK,
    PUBLISH { duplicateDelivery: bool, QoS: u8, retain: bool },
    PUBACK,
    PUBREC,
    PUBREL,
    PUBCOMP,
    SUBSCRIBE,
    SUBACK,
    UNSUBSCRIBE,
    UNSUBACK,
    PINGREQ,
    PINGRESP,
    DISCONNECT
}

impl CtrlPacket {
    fn as_bytes(&self) -> &[u8] {
        unimplemented!()
    }
}

trait HandlesMessage {
    fn handleMessage(&self, topic: &str, message: &str);
}

struct Mqtt {
    stream: TcpStream,
    user: String,
    password: String
}

impl Mqtt {
    fn new(host: &str, user: &str, password: &str) -> Mqtt {
        Mqtt {
            stream: TcpStream::connect(host).unwrap(),
            user: user.to_string(),
            password: password.to_string()
        }
    }

    fn publish(&mut self, topic: &str, text: &str) -> bool {
        // TODO implement
        let _ = &self.stream.write(text.as_bytes());
        true
    }

    fn subscribe<T: HandlesMessage>(&self, topic: &str, handler: T) -> bool {
        unimplemented!()
    }
}

fn main() {
    let mut m = Mqtt::new("localhost:1883", "host", "password");
    m.publish("test", "test");
}
