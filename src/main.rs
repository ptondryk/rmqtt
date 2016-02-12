extern crate time;

use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::sync::{Mutex, Arc};
use std::str;
use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::HashMap;

use mqtt::*;
mod mqtt;

trait HandlesMessage : Sync + Send {
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

    fn subscribe<T: HandlesMessage+'static>(&mut self, topic: &str, handler: T) {
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
    packet_id: i16,
    message_handlers: HashMap<String, Box<HandlesMessage>>,
    keep_alive: i16,
    last_message_sent: i64
}

impl MqttConnection {

    fn connect(host: &str, settings: Option<MqttConnectionSettings>)
            -> (Arc<Mutex<MqttConnection>>, JoinHandle<()>) {
        let stream = TcpStream::connect(host).unwrap();
        stream.set_read_timeout(Some(Duration::new(0, 10000)));
        let current_timestamp_second = time::get_time().sec;
        let mut mqtt_connection = Arc::new(
                        Mutex::new(MqttConnection {
                                packet_id: 0,
                                stream: Some(stream),
                                message_handlers: HashMap::new(),
                                // TODO set keep-alive properly
                                keep_alive: 120,
                                last_message_sent: current_timestamp_second
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
                shared_mqtt_connection.lock().unwrap().send(&CtrlPacket::new_connect_with_authentication("testClientId",
                    &mqtt_settings.user, &mqtt_settings.password).as_bytes().into_boxed_slice());
            }, None => {}
        }

        // return the connection object and the join handle
        (mqtt_connection.clone(), join_handler)
    }

    fn subscribe<T: HandlesMessage+'static>(&mut self, topic: &str, handler: T) {
        // send SUBSCRIBE packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&CtrlPacket::new_subscribe(topic, 0, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
        self.message_handlers.insert(topic.to_string(), Box::new(handler));
    }

    fn unsubscribe(&mut self, topic: &str) {
        // send UNSUBSCRIBE packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&CtrlPacket::new_unsubscribe(topic, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
    }

    fn publish(&mut self, topic: &str, payload: &str) {
        // send PUBLISH packet to mqtt-broker
        let new_packet_id = self.packet_id;
        self.send(&CtrlPacket::new_publish(topic, payload, new_packet_id).as_bytes().into_boxed_slice());
        self.packet_id = self.packet_id + 1;
    }

    fn send(&mut self, bytes: &[u8]) -> bool {
        match self.stream {
            Some(ref mut tcp_stream) => {
                let _ = tcp_stream.write(bytes);

                // set the last-message timestamp to now
                self.last_message_sent = time::get_time().sec;
                true
            },
            None => {
                panic!("Can't subscribe! Connect first!");
                false
            }
        }
    }

    fn disconnect(&mut self) {
        // send DISCONNECT packet to mqtt-broker
        self.send(&(CtrlPacket::DISCONNECT).as_bytes().into_boxed_slice());
    }

    fn start_receive_thread(mqtt_connection: Arc<Mutex<MqttConnection>>) {
        loop {
            match MqttConnection::try_receive_packet(mqtt_connection.clone()) {
                Some(packet) => {
                    match packet {
                        CtrlPacket::PUBLISH { packet_id, ref topic, ref payload,
                                duplicate_delivery, QoS, retain } => {
                            match mqtt_connection.lock() {
                                Ok(ref mut locked_mqtt_connection) => {
                                    if QoS == 1 {
                                        match locked_mqtt_connection.stream {
                                            Some(ref mut tcp_stream) => {
                                                tcp_stream.write(&(CtrlPacket::PUBREC {
                                                    packet_id: packet_id
                                                }).as_bytes().into_boxed_slice());
                                            },
                                            None => {}
                                        }
                                    } else if QoS == 2 {
                                        match locked_mqtt_connection.stream {
                                            Some(ref mut tcp_stream) => {
                                                tcp_stream.write(&(CtrlPacket::PUBREL {
                                                    packet_id: packet_id
                                                }).as_bytes().into_boxed_slice());
                                            },
                                            None => {}
                                        }
                                    }
                                    // TODO if QoS is 2, call handle_message not until PUBCOMP is received
                                    match locked_mqtt_connection.message_handlers.get(topic) {
                                        Some(handler) => {
                                            handler.handle_message(topic, payload);
                                        },
                                        None => {
                                            println!("No handler for {:?} registered.", topic);
                                        }
                                    }
                                },
                                Err(_) => {}
                            }
                        },
                        CtrlPacket::PUBREC { packet_id } => {
                            // TODO verify that a publish with this packet_id has been received
                            match mqtt_connection.lock() {
                                Ok(ref mut locked_mqtt_connection) => {
                                    match locked_mqtt_connection.stream {
                                        Some(ref mut tcp_stream) => {
                                            tcp_stream.write(&(CtrlPacket::PUBREL {
                                                packet_id: packet_id
                                            }).as_bytes().into_boxed_slice());
                                        },
                                        None => {}
                                    }
                                },
                                Err(_) => {}
                            }
                        },
                        CtrlPacket::PUBREL { packet_id } => {
                            // TODO verify that a publish with this packet_id has been sent
                            match mqtt_connection.lock() {
                                Ok(ref mut locked_mqtt_connection) => {
                                    match locked_mqtt_connection.stream {
                                        Some(ref mut tcp_stream) => {
                                            tcp_stream.write(&(CtrlPacket::PUBCOMP {
                                                packet_id: packet_id
                                            }).as_bytes().into_boxed_slice());
                                        },
                                        None => {}
                                    }
                                },
                                Err(_) => {}
                            }
                        },
                        _ => {}
                    }
                },
                None => {}
            }

            // keep alive check
            match mqtt_connection.lock() {
                Ok(ref mut locked_mqtt_connection) => {
                    let current_timestamp_second = time::get_time().sec;
                    if current_timestamp_second > locked_mqtt_connection.last_message_sent
                                                    + locked_mqtt_connection.keep_alive as i64 {
                        locked_mqtt_connection.send(&(CtrlPacket::PINGREQ)
                                .as_bytes().into_boxed_slice());
                    }
                },
                Err(_) => {}
            }
            thread::sleep(Duration::from_millis(3000));
        }
    }

    fn try_receive_packet(mqtt_connection: Arc<Mutex<MqttConnection>>) -> Option<CtrlPacket> {
        match mqtt_connection.lock() {
            Ok(ref locked_mqtt_connection) => {
                match locked_mqtt_connection.stream {
                    Some(ref tcp_stream) => {
                        let mut buffer: Vec<u8> = Vec::new();
                        let mut packet: Option<CtrlPacket> = None;
                        for byte in tcp_stream.bytes() {
                            match byte {
                                Ok(received_byte) => {
                                    buffer.push(received_byte);
                                    packet = mqtt::parse(&buffer);
                                },
                                Err(error) => {
                                    break;
                                }
                            }
                        }
                        buffer.clear();
                        packet
                    },
                    None => None
                }
            },
            Err(_) => None
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
