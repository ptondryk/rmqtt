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
        match self.connection_wrapper.lock() {
            Ok(mut locked_connection) => {
                locked_connection.subscribe(topic, handler);
            },
            Err(error) => {
                println!("{:?}", error);
            }
        }
    }

    fn publish(&mut self, topic: &str, payload: &str) {
        match self.connection_wrapper.lock() {
            Ok(mut locked_connection) => {
                locked_connection.publish(topic, payload);
            },
            Err(error) => {
                println!("{:?}", error);
            }
        }
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
        stream.set_read_timeout(Some(Duration::new(0, 10000)));
        let mut mqtt_connection = Arc::new(
                        Mutex::new(MqttConnection {
                                packet_id: 0,
                                stream: Some(stream)
                            }
                        ));

        // start the receive-thread
        let mut child_mqtt_connection = mqtt_connection.clone();
        let join_handler = thread::spawn(move || {
            MqttConnection::start_receive_thread(child_mqtt_connection);
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
        (mqtt_connection.clone(), join_handler)
    }

    fn subscribe<T: HandlesMessage>(&mut self, topic: &str, handler: T) {
        // send SUBSCRIBE packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&SUBSCRIBE::new(topic, 0, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
    }

    fn publish(&mut self, topic: &str, payload: &str) {
        // send PUBLISH packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&PUBLISH::new(topic, payload, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
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

    fn start_receive_thread(mqtt_connection: Arc<Mutex<MqttConnection>>) {
        loop {
            let mut locked_mqtt_connection = mqtt_connection.lock().unwrap();
            match locked_mqtt_connection.stream {
                Some(ref mut tcp_stream) => {
                    let mut buffer: Vec<u8> = Vec::new();
                    for byte in tcp_stream.bytes() {
                        match byte {
                            Ok(received_byte) => {
                                buffer.push(received_byte);
                                let received_packet = mqtt::parse(&buffer);
                                match received_packet {
                                    Some(ref packet) => {
                                        println!("parsed packet");
                                        println!("{:?}", packet);
                                        buffer.clear();
                                    },
                                    None => {}
                                }
                            },
                            Err(error) => {
                                break;
                            }
                        }
                    }
                },
                None => {}
            }
            thread::sleep(Duration::from_millis(3000));
        }
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
    m.publish("testTopic2", "lalalalalala");
    m.read_forever();
}
