extern crate time;

use std::net::TcpStream;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::thread;
use std::str;
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
    received: Vec<ReceivedMessage>,
    published: HashMap<i16, PublishToken>,
    subscribed: HashMap<i16, SubscribeToken>,
    unsubscribed: HashMap<i16, UnsubscribeToken>
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

enum PublishResult {
    Ready,
    NotComplete {
        packet_id: i16
    }
}

enum RmqttError {
    Timeout
}

struct ReceivedMessage {
    topic: String,
    payload: Vec<u8>
}

struct SubscribeToken {
    return_code: Option<u8>
}

struct UnsubscribeToken {
    unsubscribed: bool
}

struct SubscribeResult {
    packet_id: i16
}

struct UnsubscribeResult {
    packet_id: i16
}

struct Received {
    packet_id: i16
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

    fn connect(self) -> Result<MqttConnection, String> {

        // connect to the mqtt broker
        match TcpStream::connect(&*self.host) {
            Ok(stream) => {
                stream.set_read_timeout(Some(Duration::new(0, 100000000)));

                // create new MqttConnection object
                let mut new_mqtt_connection = MqttConnection {
                    packet_id: 0,
                    stream: stream,
                    keep_alive: self.keep_alive,
                    last_message_sent: time::get_time().sec,
                    received: Vec::new(),
                    published: HashMap::new(),
                    subscribed: HashMap::new(),
                    unsubscribed: HashMap::new()
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
                match new_mqtt_connection.receive(None) {
                    Ok(packet) => {
                        match packet {
                            CtrlPacket::CONNACK {session_present, return_code} => {
                                match return_code {
                                    0x00 => {
                                        Ok(new_mqtt_connection)
                                    },
                                    0x01 => {
                                        Err(String::from("Connection Refused, unacceptable protocol version"))
                                    },
                                    0x02 => {
                                        Err(String::from("Connection Refused, identifier rejected"))
                                    },
                                    0x03 => {
                                        Err(String::from("Connection Refused, Server unavailable"))
                                    },
                                    0x04 => {
                                        Err(String::from("Connection Refused, bad user name or password"))
                                    },
                                    0x05 => {
                                        Err(String::from("Connection Refused, not authorized"))
                                    },
                                    _ => {
                                        Err(String::from("Connection Refused, invalid return code"))
                                    }
                                }
                            },
                            _ => {
                                // TODO is it possible? is it error?
                                Err(String::from("Unexpected packet received"))
                            }
                        }
                    },
                    Err(error) => {
                        Err(error.to_string())
                    }
                }
            }, Err(error) => {
                Err(error.to_string())
            }
        }
    }
}

impl MqttConnection {

    // send SUBSCRIBE packet to mqtt-broker
    fn subscribe(&mut self, topic: &str, qos: u8) -> SubscribeResult {
        let next_packet_id = self.next_packet_id();
        self.send(CtrlPacket::new_subscribe(topic, qos, next_packet_id));
        self.subscribed.insert(next_packet_id, SubscribeToken {
            return_code: None
        });
        SubscribeResult {
            packet_id: next_packet_id
        }
    }

    // send UNSUBSCRIBE packet to mqtt-broker
    fn unsubscribe(&mut self, topic: &str) -> UnsubscribeResult {
        let next_packet_id = self.next_packet_id();
        self.send(CtrlPacket::new_unsubscribe(topic, next_packet_id));
        self.unsubscribed.insert(next_packet_id, UnsubscribeToken {
            unsubscribed: false
        });
        UnsubscribeResult {
            packet_id: next_packet_id
        }
    }

    // send PUBLISH packet to mqtt-broker
    fn publish(&mut self, topic: &str, payload: Vec<u8>, qos: u8) -> PublishResult {
        match qos {
            1 | 2 => {
                let next_packet_id = self.next_packet_id();
                self.published.insert(next_packet_id, PublishToken {
                    topic: topic.to_string(),
                    payload: payload.clone(),
                    publish_state: PublishState::Sent,
                    qos: qos,
                    last_message: 0
                });
                self.send(CtrlPacket::new_publish(topic, payload, next_packet_id, qos));
                PublishResult::NotComplete { packet_id: next_packet_id }
            }, _ => {
                self.send(CtrlPacket::new_publish_qos0(topic, payload));
                PublishResult::Ready
            }
        }
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

    fn await_subscribe_completed(&mut self, packet_id: i16) -> Result<u8, String> {
        loop {
            match self.subscribed.get(&packet_id) {
                Some(subscribe_token) => {
                    match subscribe_token.return_code {
                        Some(code) => {
                            return Ok(code);
                        } _ => {}
                    }
                }, None => {}
            }
            loop {
                match self.await_event(None) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_subscribe_completed_with_timeout(&mut self, packet_id: i16, timeout: &Duration)
            -> Result<u8, String> {
        let finish_timestamp_sec = time::get_time().sec + timeout.as_secs() as i64;
        loop {
            match self.subscribed.get(&packet_id) {
                Some(subscribe_token) => {
                    match subscribe_token.return_code {
                        Some(code) => {
                            return Ok(code);
                        } _ => {}
                    }
                }, None => {}
            }
            loop {
                match self.await_event(Some(finish_timestamp_sec)) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_unsubscribe_completed(&mut self, packet_id: i16) -> Result<bool, String> {
        loop {
            match self.unsubscribed.get(&packet_id) {
                Some(unsubscribe_token) => {
                    return Ok(unsubscribe_token.unsubscribed);
                }, None => {}
            }
            loop {
                match self.await_event(None) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_unsubscribe_completed_with_timeout(&mut self, packet_id: i16, timeout: &Duration)
            -> Result<bool, String> {
        let finish_timestamp_sec = time::get_time().sec + timeout.as_secs() as i64;
        loop {
            match self.unsubscribed.get(&packet_id) {
                Some(unsubscribe_token) => {
                    return Ok(unsubscribe_token.unsubscribed);
                }, None => {}
            }
            loop {
                match self.await_event(Some(finish_timestamp_sec)) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_new_message(&mut self) -> Result<ReceivedMessage, String> {
        loop {
            match self.received.pop() {
                Some(received_message) => {
                    // TODO adjust packet id to the packet id from received message
                    // TODO return message only if it has been already acknowlegded/completed (qos 1/2)
                    return Ok(received_message);
                }, None => {}
            }
            loop {
                match self.await_event(None) {
                    Ok(received_event) => {
                        break;
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    // timeout considers only second-part of the Duration
    // TODO should I check the nanosecond part too?
    fn await_new_message_with_timeout(&mut self, timeout: &Duration)
            -> Result<ReceivedMessage, String> {
        let finish_timestamp_sec = time::get_time().sec + timeout.as_secs() as i64;
        loop {
            match self.received.pop() {
                Some(received_message) => {
                    // TODO return message only if it has been already acknowlegded/completed (qos 1/2)
                    return Ok(received_message);
                }, None => {}
            }
            loop {
                match self.await_event(Some(finish_timestamp_sec)) {
                    Ok(received_event) => {
                        break;
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_publish_completion(&mut self, packet_id: i16) -> Result<PublishResult, String> {
        loop {
            match self.published.get(&packet_id) {
                Some(published_token) => {
                    match published_token.publish_state {
                        PublishState::Acknowledgement => {
                            // TODO qos is hopefully 1 in this case
                            return Ok(PublishResult::Ready);
                        },
                        PublishState::Complete => {
                            // TODO qos is hopefully 2 in this case
                            return Ok(PublishResult::Ready);
                        },
                        _ => {}
                    }
                }, None => {
                    // no message with given id sent
                    // TODO what should I do in this case?
                    return Ok(PublishResult::Ready);
                }
            }
            loop {
                match self.await_event(None) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    fn await_publish_completion_with_timeout(&mut self, packet_id: i16, timeout: &Duration)
            -> Result<PublishResult, String> {
        let finish_timestamp_sec = time::get_time().sec + timeout.as_secs() as i64;
        loop {
            match self.published.get(&packet_id) {
                Some(published_token) => {
                    match published_token.publish_state {
                        PublishState::Acknowledgement => {
                            // TODO qos is hopefully 1 in this case
                            return Ok(PublishResult::Ready);
                        },
                        PublishState::Complete => {
                            // TODO qos is hopefully 2 in this case
                            return Ok(PublishResult::Ready);
                        },
                        _ => {}
                    }
                }, None => {
                    // no message with given id sent
                    // TODO what should I do in this case?
                    return Ok(PublishResult::Ready);
                }
            }
            loop {
                match self.await_event(Some(finish_timestamp_sec)) {
                    Ok(received_event) => {
                        if received_event.packet_id == packet_id {
                            break;
                        }
                    }, Err(error) => {
                        return Err(error.to_string());
                    }
                }
            }
        }
    }

    // timeout - time to which this method should end
    fn await_event(&mut self, timeout: Option<i64>) -> Result<Received, String> {
        loop {
            match self.receive(timeout) {
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
                            self.received.push(ReceivedMessage::new(topic, payload));
                            return Ok(Received {
                                packet_id: packet_id.unwrap()
                            });
                        },
                        // TODO what to do if I receive PUBACK/COMP/.. with id that I dont know?
                        // TODO is it possible?
                        CtrlPacket::PUBACK { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state = PublishState::Acknowledgement;
                                }, None => {}
                            }
                            return Ok(Received {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::PUBREC { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state = PublishState::Received;
                                }, None => {}
                            }
                            self.send(CtrlPacket::PUBREL {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::PUBREL { packet_id } => {
                            self.send(CtrlPacket::PUBCOMP {
                                packet_id: packet_id
                            });
                        }
                        CtrlPacket::PUBCOMP { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state =
                                        PublishState::Complete
                                }, None => {}
                            }
                            return Ok(Received {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::SUBACK { packet_id, return_code } => {
                            match self.subscribed.get_mut(&packet_id) {
                                Some(mut subscribe_token) => {
                                    subscribe_token.return_code = Some(return_code);
                                }, None => {}
                            }
                            return Ok(Received {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::UNSUBACK { packet_id } => {
                            match self.unsubscribed.get_mut(&packet_id) {
                                Some(mut unsubscribe_token) => {
                                    unsubscribe_token.unsubscribed = true;
                                }, None => {}
                            }
                        }
                        _ => {}
                    }
                },
                Err(error) => {
                    return Err(error.to_string());
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

    // TODO reconnect if error occured because of connection lost
    fn receive(&mut self, timeout: Option<i64>) -> Result<CtrlPacket, String> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut single_byte = [0u8; 1];
        loop {
            match self.stream.read(&mut single_byte) {
                Ok(loaded_bytes_size) => {
                    if loaded_bytes_size > 0 {
                        // TODO fix packet receiving
                        buffer.push(single_byte[0]);
                        match mqtt::parse(&mut buffer) {
                            Some(packet) => {
                                return Ok(packet);
                            }, None => {}
                        }
                    }
                },
                Err(error) => {
                    match error.kind() {
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => {},
                        _ => {
                            return Err(error.to_string());
                        }
                    }
                }
            }
            // TODO optimize the timeout check
            match timeout {
                Some(finish_timestamp_sec) => {
                    if time::get_time().sec >= finish_timestamp_sec {
                        return Err(String::from("Timeout"));
                    }
                }, None => {}
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
        Ok(ref mut mqtt_connection) => {
            mqtt_connection.subscribe("testTopic1", 0 as u8);
            mqtt_connection.publish("testTopic2", "lalalalalala".to_string().into_bytes(), 0);
            loop {
                match mqtt_connection.await_new_message_with_timeout(&Duration::new(5, 0)) {
                    Ok(message) => {
                        println!("topic = {:?}, payload = {:?}", message.topic,
                            String::from_utf8(message.payload).unwrap());
                    }, Err(error_message) => {
                        println!("{:?}", error_message);
                    }
                }
            }
        },
        Err(message) => {
            println!("Connection failed, cause: {:?}", message);
        }
    }
}
