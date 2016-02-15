extern crate time;

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
    fn handle_message(&self, mqtt_connection: &mut MqttConnection, topic: &str, message: &str);
}

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
    message_handler: Option<Box<HandlesMessage>>,
    keep_alive: i16,
    last_message_sent: i64
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

    fn credentials(&mut self, user: &str, password: &str) -> &mut MqttConnectionBuilder {
        self.user = Some(user.to_string());
        self.password = Some(password.to_string());
        self
    }

    fn will_message(&mut self, will_topic: &str, will_content:
            &str, will_qos: u8, will_retain: bool) -> &mut MqttConnectionBuilder {
        self.will_retain = Some(will_retain);
        self.will_qos = Some(will_qos);
        self.will_topic = Some(will_topic.to_string());
        self.will_content = Some(will_content.to_string());
        self
    }

    // keep_alive in seconds
    fn keep_alive(&mut self, keep_alive: i16) -> &mut MqttConnectionBuilder {
        self.keep_alive = keep_alive;
        self
    }

    fn clean_session(&mut self) -> &mut MqttConnectionBuilder {
        self.clean_session = true;
        self
    }

    fn connect<T: HandlesMessage+'static>(&self, handler: T) -> Result<MqttConnection, &str> {
        let stream = TcpStream::connect(&*self.host).unwrap();
        stream.set_read_timeout(Some(Duration::new(0, 10000)));

        let connect: CtrlPacket = CtrlPacket::CONNECT {
            clientId: self.client_id.clone(),
            topic: self.will_topic.clone(),
            content: self.will_content.clone(),
            qos: self.will_qos.clone(),
            retain: self.will_retain.clone(),
            username: self.user.clone(),
            password: self.password.clone(),
            clean_session: self.clean_session,
            keep_alive: self.keep_alive
        };

        let mut new_mqtt_connection = MqttConnection {
            packet_id: 0,
            stream: stream,
            message_handler: Some(Box::new(handler)),
            keep_alive: self.keep_alive,
            last_message_sent: time::get_time().sec
        };

        new_mqtt_connection.send(connect);
        match new_mqtt_connection.receive() {
            Ok(received_packet) => {
                // TODO verify that received packet is "successful" CONNACK
            }, Err(_) => {
                // TODO connect should return Err in this case
            }
        }

        Ok(new_mqtt_connection)
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
    fn publish(&mut self, topic: &str, payload: &str) {
        let next_packet_id = self.next_packet_id();
        self.send(CtrlPacket::new_publish(topic, payload, next_packet_id));
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

    fn block_main_thread_and_receive(&mut self) {
        loop {
            match self.receive() {
                Ok(packet) => {
                    match packet {
                        CtrlPacket::PUBLISH { packet_id, ref topic, ref payload,
                                    duplicate_delivery, qos, retain } => {
                            if qos == 1 {
                                self.send(CtrlPacket::PUBREC {
                                    packet_id: packet_id
                                });
                            } else if qos == 2 {
                                self.send(CtrlPacket::PUBREL {
                                    packet_id: packet_id
                                });
                            }
                            // TODO if qos is 2, call handle_message not until PUBCOMP is received
                            self.forward_message_to_handler(topic, payload);

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
                Err(_) => {}
            }

            // keep alive check
            let current_timestamp_second = time::get_time().sec;
            if current_timestamp_second > self.last_message_sent + self.keep_alive as i64 {
                self.send(CtrlPacket::PINGREQ);
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    fn receive(&mut self) -> Result<CtrlPacket, &str> {
        let mut buffer: Vec<u8> = Vec::new();
        let mut packet: Option<CtrlPacket> = None;
        while packet.is_none() {
            self.stream.read_to_end(&mut buffer);
            packet = mqtt::parse(&buffer);
            thread::sleep(Duration::from_millis(500));
        }
        Ok(packet.unwrap())
    }

    fn forward_message_to_handler(&mut self, topic: &str, payload: &str) {
        match self.message_handler {
            Some(ref mut handler) => {
                // TODO implement
                // handler.handle_message(self, topic, payload);
            }, None => {}
        }
    }
}

struct ExampleHandler {
    example_var: i16
}

impl ExampleHandler {

}

impl HandlesMessage for ExampleHandler {
    fn handle_message(&self, mqtt_connection: &mut MqttConnection, topic: &str, message: &str) {
        println!("topic: {:?}, message: {:?}", topic, message);
    }
}

fn main() {
    match MqttConnectionBuilder::new("test-client-01", "localhost:1883")
            .credentials("system", "manager")
            .keep_alive(120)
            .connect(ExampleHandler {
                example_var: 1
            }) {
        Ok(ref mut mqtt_connection) => {
            mqtt_connection.subscribe("testTopic1", 0 as u8);
            mqtt_connection.publish("testTopic2", "lalalalalala");
            mqtt_connection.block_main_thread_and_receive();
        },
        Err(_) => {}
    }
}
