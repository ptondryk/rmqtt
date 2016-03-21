extern crate time;

use std::net::TcpStream;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::time::Duration;
use std::collections::HashMap;
use time::Timespec;
use std::ops::Add;
use std::ops::Sub;

use mqtt::*;
mod mqtt;

/// A structure that can be used to connect to mqtt-broker and build a session.
///
/// # Examples
///
/// ```
/// use rmqtt::MqttSessionBuilder;
///
/// match MqttSessionBuilder::new("test-client", "localhost:1883")
///         .credentials("user", "password")
///         .keep_alive(120)
///         .connect() {
///     Ok(ref mut mqtt_session) => {
///         // do somthing with session
///     },
///     Err(message) => {
///         // session creation failed (cause can be found in message)
///     }
/// }
/// ```
pub struct MqttSessionBuilder {
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

/// This structure represents a mqtt session. It can be created using `MqttSessionBuilder`.
pub struct MqttSession {
    host: String,
    packet_id: i16,
    connection: MqttConnection,
    keep_alive: i16,
    received: Vec<ReceivedMessage>,
    published: HashMap<i16, PublishToken>,
    subscribed: HashMap<i16, SubscribeToken>,
    unsubscribed: HashMap<i16, UnsubscribeToken>
}

/// Message received from broker. It contains information about `topic` to which this
/// message has been sent and `payload` of this message as bytes vector.
pub struct ReceivedMessage {
    pub topic: String,
    pub payload: Vec<u8>
}

/// `PublishResult` of messages published with quality of service 0 is always `Ready` (the
/// publishing is complete). Other messages must be acknowledged by broker. In this case
/// the result of publishing must by checked using method `await_publish_completion`. This method
/// uses `packet_id` contained in `PublishResult::NotComplete`, returned by `publish` method,
/// as its argument.
pub enum PublishResult {
    /// Messages published with quality of service 0 are always `Ready`.
    Ready,
    NotComplete {
        packet_id: i16
    }
}

/// Result of `subscribe` method.
pub struct SubscribeResult {
    pub packet_id: i16
}

/// Result of `unsubscribe` method.
pub struct UnsubscribeResult {
    pub packet_id: i16
}

/// Errors that can occure while trying to connect to mqtt broker.
pub enum ConnectFailed {
    UnexpectedPacket,
    UnacceptableProtocol,
    IdentifierRejected,
    ServerUnavailable,
    BadCredentials,
    NotAuthorized,
    InvalidReturnCode,
    ConnectionError {
        details: String
    }
}

/// Possible results of the `MqttSession.await_*` methods.
pub enum ReceiveFailed {
    Timeout,
    ConnectionError {
        details: String
    }
}

struct MqttConnection {
    stream: TcpStream,
    last_message_sent: i64
}

struct PublishToken {
    topic: String,
    payload: Vec<u8>,
    publish_state: PublishState,
    qos: u8
    // TODO is this necessary?
    // last_message: i64
}

enum PublishState {
    Sent,
    Acknowledgement,
    Received,
    Release,
    Complete
}

struct SubscribeToken {
    return_code: Option<u8>
}

struct UnsubscribeToken {
    unsubscribed: bool
}

enum ReceivedPacket {
    WithId {
        packet_id: i16
    },
    WithoutId
}

impl MqttSessionBuilder {

    /// Creates a new `MqttSessionBuilder`.
    pub fn new(client_id: &str, host: &str) -> MqttSessionBuilder {
        MqttSessionBuilder {
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

    /// This method can be used to specify the username and password
    /// that should be used to authenticated the client when connecting to
    /// the mqtt broker.
    ///
    /// If this method is not called then the `MqttSessionBuilder` tries to create
    /// a connection without authntication.
    pub fn credentials(mut self, user: &str, password: &str) -> MqttSessionBuilder {
        self.user = Some(user.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// This method can be used to define the Will Message. Will Message is
    /// (acording to the mqtt 3.1.1 documentation) a message that will be published to the
    /// in this method specified topic when the unexpected connection lost between
    /// client and broker occures.
    ///
    /// If this method is not called then no Will Message is defined.
    pub fn will_message(mut self, will_topic: &str, will_content: &str,
            will_qos: u8, will_retain: bool) -> MqttSessionBuilder {
        self.will_retain = Some(will_retain);
        self.will_qos = Some(will_qos);
        self.will_topic = Some(will_topic.to_string());
        self.will_content = Some(will_content.to_string());
        self
    }

    /// This method can be used to define how often should the broker expect a
    /// keep alive message from client. The time period is defined in seconds.
    pub fn keep_alive(mut self, keep_alive: i16) -> MqttSessionBuilder {
        self.keep_alive = keep_alive;
        self
    }

    /// Call of this method indicates that a clean session should be initialized.
    pub fn clean_session(mut self) -> MqttSessionBuilder {
        self.clean_session = true;
        self
    }

    /// This method can be used to initialize the connetion between broker and client using
    /// parameters defined in this `MqttSessionBuilder`.
    #[allow(unused_variables)]
    pub fn connect(self) -> Result<MqttSession, ConnectFailed> {

        // connect to the mqtt broker
        match TcpStream::connect(&*self.host) {
            Ok(stream) => {
                let new_mqtt_connection = MqttConnection {
                    stream: stream,
                    last_message_sent: time::get_time().sec
                };

                // create new MqttSession object
                let mut new_mqtt_session = MqttSession {
                    host: self.host,
                    packet_id: 0,
                    connection: new_mqtt_connection,
                    keep_alive: self.keep_alive,
                    received: Vec::new(),
                    published: HashMap::new(),
                    subscribed: HashMap::new(),
                    unsubscribed: HashMap::new()
                };

                // send CONNECT packet to mqtt broker
                new_mqtt_session.connection.send(CtrlPacket::CONNECT {
                    client_id: self.client_id,
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
                match new_mqtt_session.connection.receive(None) {
                    Ok(packet) => {
                        match packet {
                            CtrlPacket::CONNACK {session_present, return_code} => {
                                match return_code {
                                    0x00 => {
                                        Ok(new_mqtt_session)
                                    },
                                    0x01 => {
                                        // Connection Refused, unacceptable protocol version
                                        Err(ConnectFailed::UnacceptableProtocol)
                                    },
                                    0x02 => {
                                        // Connection Refused, identifier rejected
                                        Err(ConnectFailed::IdentifierRejected)
                                    },
                                    0x03 => {
                                        // Connection Refused, Server unavailable
                                        Err(ConnectFailed::ServerUnavailable)
                                    },
                                    0x04 => {
                                        // Connection Refused, bad user name or password
                                        Err(ConnectFailed::BadCredentials)
                                    },
                                    0x05 => {
                                        // Connection Refused, not authorized
                                        Err(ConnectFailed::NotAuthorized)
                                    },
                                    _ => {
                                        // Connection Refused, invalid return code
                                        Err(ConnectFailed::InvalidReturnCode)
                                    }
                                }
                            },
                            _ => {
                                // TODO is it possible? is it error?
                                Err(ConnectFailed::UnexpectedPacket)
                            }
                        }
                    },
                    Err(error) => {
                        Err(ConnectFailed::ConnectionError {
                            details: String::from("No packet received")
                        })
                    }
                }
            }, Err(error) => {
                Err(ConnectFailed::ConnectionError {
                    details: error.to_string()
                })
            }
        }
    }
}

impl MqttSession {

    /// When connection is lost, this method can be used to reconnect.
    // TODO test this method...
    #[allow(unused_must_use)]
    pub fn reconnect(&mut self) -> bool {
        match TcpStream::connect(&*self.host) {
            Ok(stream) => {
                stream.set_read_timeout(Some(Duration::new(0, 100000000)));
                // TODO does it depend on clean_session flag?
                self.redeliver_packets();
                true
            },
            Err(_) => {
                false
            }
        }
    }

    /// This method sends a SUBSCRIBE packet to mqtt-broker.
    /// # Note
    /// Calling this method does not mean that the topic is immediately subscribed.
    /// To make sure that the client subscribed successfully to topic method
    /// `await_subscribe_completed` should be used.
    pub fn subscribe(&mut self, topic: &str, qos: u8) -> SubscribeResult {
        let next_packet_id = self.next_packet_id();
        self.connection.send(CtrlPacket::new_subscribe(topic, qos, next_packet_id));
        self.subscribed.insert(next_packet_id, SubscribeToken {
            return_code: None
        });
        SubscribeResult {
            packet_id: next_packet_id
        }
    }

    /// This method sends an UNSUBSCRIBE packet to mqtt-broker.
    /// # Note
    /// Calling this method does not mean that the topic is immediately unsubscribed.
    /// To make sure that the client unsubscribed the topic successfully method
    /// `await_unsubscribe_completed` should be used.
    pub fn unsubscribe(&mut self, topic: &str) -> UnsubscribeResult {
        let next_packet_id = self.next_packet_id();
        self.connection.send(CtrlPacket::new_unsubscribe(topic, next_packet_id));
        self.unsubscribed.insert(next_packet_id, UnsubscribeToken {
            unsubscribed: false
        });
        UnsubscribeResult {
            packet_id: next_packet_id
        }
    }

    /// This method sends a PUBLISH packet to mqtt-broker.
    /// # Note
    /// Calling this method does not mean that a message is immediately published.
    /// To make sure that the message has been successfully published to the topic, method
    /// `await_publish_completion` should be used.
    pub fn publish(&mut self, topic: &str, payload: Vec<u8>, qos: u8) -> PublishResult {
        match qos {
            1 | 2 => {
                let next_packet_id = self.next_packet_id();
                self.published.insert(next_packet_id, PublishToken {
                    topic: topic.to_string(),
                    payload: payload.clone(),
                    publish_state: PublishState::Sent,
                    qos: qos
                });
                self.connection.send(CtrlPacket::new_publish(topic, payload, next_packet_id, qos));
                PublishResult::NotComplete { packet_id: next_packet_id }
            }, _ => {
                self.connection.send(CtrlPacket::new_publish_qos0(topic, payload));
                PublishResult::Ready
            }
        }
    }

    /// This method sends DISCONNECT packet to mqtt-broker.
    pub fn disconnect(&mut self) {
        self.connection.send(CtrlPacket::DISCONNECT);
    }

    /// Method waits (blocks the execution) until the topic subscription is completed.
    pub fn await_subscribe_completed(&mut self, subscribe_packet_id: i16,
            timeout: Option<Duration>) -> Result<u8, ReceiveFailed> {
        let finish_timestamp: Option<Timespec> =
            timeout.map(|timeout| time::get_time().add(time::Duration::nanoseconds(
                (timeout.as_secs() * 1000000000 + timeout.subsec_nanos() as u64) as i64)));
        loop {
            match self.subscribed.get(&subscribe_packet_id) {
                Some(subscribe_token) => {
                    match subscribe_token.return_code {
                        Some(code) => {
                            return Ok(code);
                        } _ => {}
                    }
                }, None => {}
            }
            loop {
                match self.await_event(finish_timestamp) {
                    Ok(received_event) => {
                        match received_event {
                            ReceivedPacket::WithId { packet_id } => {
                                if packet_id == subscribe_packet_id {
                                    break;
                                }
                            },
                            _ => {}
                        }
                    }, Err(error) => {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Method waits (blocks the execution) until the topic is
    /// successfully unsubscribed.
    pub fn await_unsubscribe_completed(&mut self, unsubscribe_packet_id: i16,
            timeout: Option<Duration>) -> Result<bool, ReceiveFailed> {
        let finish_timestamp: Option<Timespec> =
            timeout.map(|timeout| time::get_time().add(time::Duration::nanoseconds(
                (timeout.as_secs() * 1000000000 + timeout.subsec_nanos() as u64) as i64)));
        loop {
            match self.unsubscribed.get(&unsubscribe_packet_id) {
                Some(unsubscribe_token) => {
                    match unsubscribe_token.unsubscribed {
                        true => { return Ok(true); },
                        false => {}
                    }
                }, None => {}
            }
            loop {
                match self.await_event(finish_timestamp) {
                    Ok(received_event) => {
                        match received_event {
                            ReceivedPacket::WithId { packet_id } => {
                                if packet_id == unsubscribe_packet_id {
                                    break;
                                }
                            },
                            _ => {}
                        }
                    }, Err(error) => {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Method waits (blocks the execution) on a new message.
    pub fn await_new_message(&mut self, timeout: Option<Duration>)
            -> Result<ReceivedMessage, ReceiveFailed> {
        let finish_timestamp: Option<Timespec> =
            timeout.map(|timeout| time::get_time().add(time::Duration::nanoseconds(
                (timeout.as_secs() * 1000000000 + timeout.subsec_nanos() as u64) as i64)));
        loop {
            match self.received.pop() {
                Some(received_message) => {
                    // TODO return message only if it has been already acknowlegded/completed (qos 1/2)
                    return Ok(received_message);
                }, None => {}
            }
            loop {
                match self.await_event(finish_timestamp) {
                    Ok(_) => {
                        break;
                    }, Err(error) => {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Method waits (blocks the execution) until the message publishing is completed.
    /// # Note
    /// timeout considers only second-part of the Duration
    // TODO should I check the nanosecond part too?
    pub fn await_publish_completion(&mut self, publish_packet_id: i16,
            timeout: Option<Duration>) -> Result<PublishResult, ReceiveFailed> {
        let finish_timestamp: Option<Timespec> =
            timeout.map(|timeout| time::get_time().add(time::Duration::nanoseconds(
                (timeout.as_secs() * 1000000000 + timeout.subsec_nanos() as u64) as i64)));
        loop {
            match self.published.get(&publish_packet_id) {
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
                match self.await_event(finish_timestamp) {
                    Ok(received_event) => {
                        match received_event {
                            ReceivedPacket::WithId { packet_id } => {
                                if packet_id == publish_packet_id {
                                    break;
                                }
                            },
                            _ => {}
                        }
                    }, Err(error) => {
                        return Err(error);
                    }
                }
            }
        }
    }

    // timeout - time when this method must end
    #[allow(unused_variables)]
    fn await_event(&mut self, timeout: Option<Timespec>) -> Result<ReceivedPacket, ReceiveFailed> {
        loop {
            match self.connection.receive(timeout) {
                Ok(packet) => {
                    match packet {
                        CtrlPacket::PUBLISH { packet_id, topic, payload,
                                    duplicate_delivery, qos, retain } => {
                            if qos == 1 {
                                self.connection.send(CtrlPacket::PUBACK {
                                    packet_id: packet_id.unwrap()
                                });
                            } else if qos == 2 {
                                self.connection.send(CtrlPacket::PUBREC {
                                    packet_id: packet_id.unwrap()
                                });
                            }
                            self.received.push(ReceivedMessage::new(topic, payload));
                            match packet_id {
                                Some(publish_packet_id) => {
                                    return Ok(ReceivedPacket::WithId {
                                        packet_id: publish_packet_id
                                    });
                                }, None => {
                                    return Ok(ReceivedPacket::WithoutId);
                                }
                            }
                        },
                        // TODO what to do if I receive PUBACK/COMP/.. with id that I dont know?
                        // TODO is it possible?
                        CtrlPacket::PUBACK { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state = PublishState::Acknowledgement;
                                }, None => {}
                            }
                            return Ok(ReceivedPacket::WithId {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::PUBREC { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state = PublishState::Received;
                                }, None => {}
                            }
                            self.connection.send(CtrlPacket::PUBREL {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::PUBREL { packet_id } => {
                            match self.published.get_mut(&packet_id) {
                                Some(mut published_token) => {
                                    published_token.publish_state =
                                        PublishState::Release
                                }, None => {}
                            }
                            self.connection.send(CtrlPacket::PUBCOMP {
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
                            return Ok(ReceivedPacket::WithId {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::SUBACK { packet_id, return_code } => {
                            match self.subscribed.get_mut(&packet_id) {
                                Some(mut subscribe_token) => {
                                    subscribe_token.return_code = Some(return_code);
                                }, None => {}
                            }
                            return Ok(ReceivedPacket::WithId {
                                packet_id: packet_id
                            });
                        },
                        CtrlPacket::UNSUBACK { packet_id } => {
                            match self.unsubscribed.get_mut(&packet_id) {
                                Some(mut unsubscribe_token) => {
                                    unsubscribe_token.unsubscribed = true;
                                }, None => {}
                            }
                            return Ok(ReceivedPacket::WithId {
                                packet_id: packet_id
                            });
                        }
                        _ => {}
                    }
                },
                Err(error) => {
                    return Err(error);
                }
            }

            // keep alive check
            let current_timestamp_second = time::get_time().sec;
            if current_timestamp_second > self.connection.last_message_sent + self.keep_alive as i64 {
                self.connection.send(CtrlPacket::PINGREQ);
            }
        }
    }

    fn redeliver_packets(&mut self) {
        for (packet_id, published_token) in self.published.iter() {
            self.connection.send(CtrlPacket::new_publish(&published_token.topic,
                published_token.payload.clone(), packet_id.clone(), published_token.qos));
        }
    }

    fn next_packet_id(&mut self) -> i16 {
        self.packet_id = self.packet_id + 1;
        self.packet_id
    }

}

impl MqttConnection {

    fn send(&mut self, ctrl_packet: CtrlPacket) {
        let bytes: &[u8] = &ctrl_packet.as_bytes().into_boxed_slice();
        let _ = self.stream.write(bytes);

        // set the last-message timestamp to now
        self.last_message_sent = time::get_time().sec;
    }

    /// Method receives a package from broker.
    #[allow(unused_must_use)]
    fn receive(&mut self, timeout: Option<Timespec>) -> Result<CtrlPacket, ReceiveFailed> {
        let mut buffer: Vec<u8> = Vec::new();
        loop {
            match timeout {
                Some(finish_timestamp) => {
                    let time_duration = finish_timestamp.sub(time::get_time());
                    let duration = Duration::new(time_duration.num_seconds() as u64,
                        (time_duration.num_nanoseconds().unwrap()
                            - time_duration.num_seconds() * 1000000000) as u32);
                    self.stream.set_read_timeout(Some(duration));
                }, None => {
                    self.stream.set_read_timeout(None);
                }
            }

            for byte in std::io::Read::by_ref(&mut self.stream).bytes() {
                match byte {
                    Ok(received_byte) => {
                        buffer.push(received_byte);
                        match mqtt::parse(&mut buffer) {
                            Some(packet) => {
                                return Ok(packet);
                            }, None => {}
                        }
                    },
                    Err(error) => {
                        match error.kind() {
                            ErrorKind::WouldBlock | ErrorKind::TimedOut => {
                                break;
                            },
                            _ => {
                                return Err(ReceiveFailed::ConnectionError {
                                    details: error.to_string()
                                });
                            }
                        }
                    }
                }
            }
            // TODO optimize the timeout check
            match timeout {
                Some(ref finish_timestamp) => {
                    if time::get_time().lt(finish_timestamp) {
                        return Err(ReceiveFailed::Timeout);
                    }
                }, None => {}
            }
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

#[cfg(test)]
mod tests {

    // TODO write tests

}
