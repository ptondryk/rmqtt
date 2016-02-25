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

struct MqttConnectionBuilder {
    client_id: String,
    host: String,
    user: Option<String>,
    password: Option<String>,
    will_topic: Option<String>,
    will_content: Option<String>,
    will_qos: Option<u8>,
    will_retain: Option<bool>,
    clean_session: bool,
    keep_alive: i16
}

struct MqttConnection {
    packet_id: i16,
    stream: TcpStream,
    keep_alive: i16,
    last_message_sent: i64,
    published: HashMap<i16, PublishToken>
}

struct PublishToken {
    topic: String,
    payload: Vec<u8>,
    publish_state: PublishState,
    qos: u8,
    last_message: i64
}

enum PublishState {
    Sent,
    Acknowledgement,
    Received,
    Release,
    Complete
}

struct ReceivedMessage {
    topic: String,
    payload: Vec<u8>
}

impl MqttConnectionBuilder {

    fn new(client_id: &str, host: &str) -> MqttConnectionBuilder {
        MqttConnectionBuilder {
            client_id: client_id.to_string(),
            host: host.to_string(),
            user: None,
            password: None,
            will_topic: None,
            will_content: None,
            will_qos: None,
            will_retain: None,
            clean_session: false,
            keep_alive: 0
        }
    }

    fn credentials(mut self, user: &str, password: &str) -> MqttConnectionBuilder {
        self.user = Some(user.to_string());
        self.password = Some(password.to_string());
        self
    }

    fn will_message(mut self, will_topic: &str, will_content: &str,
            will_qos: u8, will_retain: bool) -> MqttConnectionBuilder {
        self.will_retain = Some(will_retain);
        self.will_qos = Some(will_qos);
        self.will_topic = Some(will_topic.to_string());
        self.will_content = Some(will_content.to_string());
        self
    }

    // keep_alive in seconds
    fn keep_alive(mut self, keep_alive: i16) -> MqttConnectionBuilder {
        self.keep_alive = keep_alive;
        self
    }

    fn clean_session(mut self) -> MqttConnectionBuilder {
        self.clean_session = true;
        self
    }

    fn connect(self) -> Result<MqttConnection, &'static str> {

        // connect to the mqtt broker
        let stream = TcpStream::connect(&*self.host).unwrap();
        stream.set_read_timeout(Some(Duration::new(0, 10000)));

        // create new MqttConnection object
        let mut new_mqtt_connection = MqttConnection {
            packet_id: 0,
            stream: stream,
            keep_alive: self.keep_alive,
            last_message_sent: time::get_time().sec,
            published: HashMap::new()
        };

        // send CONNECT packet to mqtt broker
        new_mqtt_connection.send(CtrlPacket::CONNECT {
            clientId: self.client_id,
            topic: self.will_topic,
            content: self.will_content,
            qos: self.will_qos,
            retain: self.will_retain,
            username: self.user,
            password: self.password,
            clean_session: self.clean_session,
            keep_alive: self.keep_alive
        });

        // try receive CONNACK
        match new_mqtt_connection.receive().unwrap() {
            CtrlPacket::CONNACK {session_present, return_code} => {
                match return_code {
                    0x00 => {
                        Ok(new_mqtt_connection)
                    },
                    0x01 => {
                        Err("Connection Refused, unacceptable protocol version")
                    },
                    0x02 => {
                        Err("Connection Refused, identifier rejected")
                    },
                    0x03 => {
                        Err("Connection Refused, Server unavailable")
                    },
                    0x04 => {
                        Err("Connection Refused, bad user name or password")
                    },
                    0x05 => {
                        Err("Connection Refused, not authorized")
                    },
                    _ => {
                        Err("Connection Refused, invalid return code")
                    }
                }
            },
            _ => {
                // TODO is it possible? is it error?
                Err("Unexpected packet received")
            }
        }
    }
}

impl MqttConnection {

    // send SUBSCRIBE packet to mqtt-broker
    fn subscribe(&mut self, topic: &str, qos: u8) {
        let next_packet_id = self.next_packet_id();
        self.send(CtrlPacket::new_subscribe(topic, qos, next_packet_id));
    }

    // send UNSUBSCRIBE packet to mqtt-broker
    fn unsubscribe(&mut self, topic: &str) {
        let next_packet_id = self.next_packet_id();
        self.send(CtrlPacket::new_unsubscribe(topic, next_packet_id));
    }

    // send PUBLISH packet to mqtt-broker
    fn publish(&mut self, topic: &str, payload: Vec<u8>, qos: u8) {
        let next_packet_id = self.next_packet_id();
        match qos {
            1 | 2 => {
                self.published.insert(next_packet_id, PublishToken {
                    topic: topic.to_string(),
                    payload: payload.clone(),
                    publish_state: PublishState::Sent,
                    qos: qos,
                    last_message: 0
                });
            }, _ => {}
        }
        self.send(CtrlPacket::new_publish(topic, payload, next_packet_id, qos));
    }

    // send DISCONNECT packet to mqtt-broker
    fn disconnect(&mut self) {
        self.send(CtrlPacket::DISCONNECT);
    }

    fn next_packet_id(&mut self) -> i16 {
        self.packet_id = self.packet_id + 1;
        self.packet_id
    }

    fn send(&mut self, ctrl_packet: CtrlPacket) {
        let bytes: &[u8] = &ctrl_packet.as_bytes().into_boxed_slice();
        let _ = self.stream.write(bytes);

        // set the last-message timestamp to now
        self.last_message_sent = time::get_time().sec;
    }

    fn receive(&mut self) -> Result<CtrlPacket, &str> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut packet: Option<CtrlPacket> = None;
        while packet.is_none() {
            // TODO read single bytes to avoid
            // that I read bytes of second packet and "forget" them
            self.stream.read_to_end(&mut buffer);
            packet = mqtt::parse(&mut buffer);
            thread::sleep(Duration::from_millis(500));
        }
        Ok(packet.unwrap())
    }

    fn await_message(&mut self) -> ReceivedMessage {
        self.block_main_thread_and_receive()
    }

    fn await_message_with_timeout(&mut self, timeout: i16) -> Option<ReceivedMessage> {
        // TODO implement timeout
        Some(self.await_message())
    }

    fn block_main_thread_and_receive(&mut self) -> ReceivedMessage {
        loop {
            match self.receive() {
                Ok(packet) => {
                    match packet {
                        CtrlPacket::PUBLISH { packet_id, topic, payload,
                                    duplicate_delivery, qos, retain } => {
                            if qos == 1 {
                                self.send(CtrlPacket::PUBACK {
                                    packet_id: packet_id.unwrap()
                                });
                            } else if qos == 2 {
                                self.send(CtrlPacket::PUBREC {
                                    packet_id: packet_id.unwrap()
                                });
                            }
                            // TODO if qos is 2, do not return message until PUBCOMP is received
                            return ReceivedMessage::new(topic, payload);
                        },
                        CtrlPacket::PUBREC { packet_id } => {
                            // TODO verify that a publish with this packet_id has been received
                            self.send(CtrlPacket::PUBREL {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::PUBREL { packet_id } => {
                            // TODO verify that a publish with this packet_id has been sent
                            self.send(CtrlPacket::PUBCOMP {
                                packet_id: packet_id
                            });
                        },
                        _ => {}
                    }
                },
                Err(_) => {
                    // TODO reconnect if error occured because of connection lost
                }
            }

            // keep alive check
            let current_timestamp_second = time::get_time().sec;
            if current_timestamp_second > self.last_message_sent + self.keep_alive as i64 {
                self.send(CtrlPacket::PINGREQ);
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

}

impl ReceivedMessage {

    fn new(topic: String, payload: Vec<u8>) -> ReceivedMessage {
        ReceivedMessage {
            topic: topic,
            payload: payload
        }
    }

}

fn main() {
    match MqttConnectionBuilder::new("test-client-01", "localhost:1883")
            .credentials("system", "manager")
            .keep_alive(120)
            .connect() {
        Ok(ref mut mqtt_session) => {
            mqtt_session.subscribe("testTopic1", 0 as u8);
            mqtt_session.publish("testTopic2", "lalalalalala".to_string().into_bytes(), 0);
            loop {
                let message = mqtt_session.await_message();
                println!("topic = {:?}, payload = {:?}", message.topic,
                    String::from_utf8(message.payload).unwrap());
            }
        },
        Err(_) => {}
    }
}
