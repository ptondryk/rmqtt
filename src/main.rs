use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::sync::{Mutex, Arc};
use std::str;
use std::thread::JoinHandle;
use std::time::Duration;

use mqtt::*;
mod mqtt;

trait HandlesMessage {
    fn handle_message(&self, topic: &str, message: &str);
}

struct Mqtt {
    join_handler: JoinHandle<()>,
    connection_wrapper: Arc<Mutex<MqttConnection>>
}

impl Mqtt {

    fn connect(address: &str, settings: Option<MqttConnectionSettings>) -> Mqtt {
        let (mqtt_connection, join_handler) = MqttConnection::connect(address, settings);
        Mqtt {
            connection_wrapper: mqtt_connection,
            join_handler: join_handler
        }
    }

    pub fn read_forever(self) {
        self.join_handler.join();
    }

    fn subscribe<T: HandlesMessage>(&mut self, topic: &str, handler: T) {
        self.connection_wrapper.lock().unwrap().subscribe(topic, handler);
    }
}

struct MqttConnection {
    stream: Option<TcpStream>,
    packet_id: i16
}

impl MqttConnection {

    fn connect(host: &str, settings: Option<MqttConnectionSettings>)
            -> (Arc<Mutex<MqttConnection>>, JoinHandle<()>) {
        let stream = TcpStream::connect(host).unwrap();
        let mut mqtt_connection = Arc::new(
                        Mutex::new(MqttConnection {
                                packet_id: 0,
                                stream: Some(stream)
                            }
                        ));

        // start the receive-thread
        let mut child_mqtt_connection = mqtt_connection.clone();
        let join_handler = thread::spawn(move || {
            loop {
                child_mqtt_connection.lock().unwrap().receive();
            }
        });

        let shared_mqtt_connection = mqtt_connection.clone();

        // send CONNECT packet to mqtt-broker
        match settings {
            Some(mqtt_settings) => {
                shared_mqtt_connection.lock().unwrap().send(&CONNECT::new_with_authentication("testClientId",
                    &mqtt_settings.user, &mqtt_settings.password).as_bytes().into_boxed_slice());
            }, None => {}
        }

        // return the connection object and the join handle
        (shared_mqtt_connection, join_handler)
    }


    fn publish(&mut self, topic: &str, payload: &str) -> bool {
        unimplemented!()
    }

    fn subscribe<T: HandlesMessage>(&mut self, topic: &str, handler: T) -> bool {
        // send SUBSCRIBE packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&SUBSCRIBE::new(topic, 0, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
        true
    }

    fn send(&mut self, bytes: &[u8]) -> bool {
        match self.stream {
            Some(ref mut tcp_stream) => {
                let _ = tcp_stream.write(bytes);
                true
            },
            None => {
                panic!("Can't subscribe! Connect first!");
                false
            }
        }
    }

    fn receive(&mut self) {
        match(self.stream) {
            Some(ref mut tcp_stream) => {
                let mut buffer: Vec<u8> = Vec::new();
                for byte in tcp_stream.bytes() {
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
        thread::sleep(Duration::from_millis(3000));
    }

}

struct MqttConnectionSettings {
    user: String,
    password: String
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
    let mut m = Mqtt::connect("localhost:1883", Some(MqttConnectionSettings {
                                                    user: "system".to_string(),
                                                    password: "manager".to_string()
                                                }));
    m.subscribe("testTopic1", ExampleHandler {
        example_var: 1
    });
    loop {}
}
