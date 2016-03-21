//! This module contains implementation of the MQTT protocol (version 3.1.1).

/// This enum represents a single mqtt control packet.
pub enum CtrlPacket {
    CONNECT {
        client_id: String,
        topic: Option<String>,
        content: Option<String>,
        qos: Option<u8>,
        retain: Option<bool>,
        username: Option<String>,
        password: Option<String>,
        clean_session: bool,
        keep_alive: i16
    },
    CONNACK {
        session_present: bool,
        return_code: u8
    },
    PUBLISH {
        packet_id: Option<i16>,
        topic: String,
        payload: Vec<u8>,
        duplicate_delivery: bool,
        qos: u8,
        retain: bool
    },
    PUBACK {
        packet_id: i16,
    },
    PUBREC {
        packet_id: i16,
    },
    PUBREL {
        packet_id: i16,
    },
    PUBCOMP {
        packet_id: i16,
    },
    SUBSCRIBE {
        topic_filter: Vec<String>,
        qos: Vec<u8>,
        packet_id: i16
    },
    SUBACK {
        packet_id: i16,
        return_code: u8
    },
    UNSUBSCRIBE {
        topic_filter: Vec<String>,
        packet_id: i16
    },
    UNSUBACK {
        packet_id: i16
    },
    PINGREQ, PINGRESP, DISCONNECT
}

impl CtrlPacket {

    pub fn new_subscribe(topic: &str, qos: u8, packet_id: i16) -> CtrlPacket {
        CtrlPacket::SUBSCRIBE {
            topic_filter: vec![topic.to_string()],
            qos: vec![qos],
            packet_id: packet_id
        }
    }

    pub fn new_unsubscribe(topic: &str, packet_id: i16) -> CtrlPacket {
        CtrlPacket::UNSUBSCRIBE {
            topic_filter: vec![topic.to_string()],
            packet_id: packet_id
        }
    }

    pub fn new_publish_qos0(topic: &str, payload: Vec<u8>) -> CtrlPacket {
        CtrlPacket::PUBLISH {
            packet_id: None,
            topic: topic.to_string(),
            payload: payload,
            qos: 0,
            // TODO set the values properly
            duplicate_delivery: false,
            retain: false
        }
    }

    pub fn new_publish(topic: &str, payload: Vec<u8>, packet_id: i16, qos: u8) -> CtrlPacket {
        // TODO verify that qos has proper value (1 or 2)
        CtrlPacket::PUBLISH {
            packet_id: Some(packet_id),
            topic: topic.to_string(),
            payload: payload,
            qos: qos,
            // TODO set the values properly
            duplicate_delivery: false,
            retain: false
        }
    }

    pub fn as_bytes(self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();

        match self {
            CtrlPacket::CONNECT { client_id, topic, content, qos, retain, username,
                        password, clean_session, keep_alive } => {
                // id = 1 and reserved flags = 0
                result.push(0x10);

                // Protocol Name ("MQTT" encoded as specified in 1.5.3)
                result.append(&mut encode_string("MQTT"));

                // Protocol Level (MQTT 3.1.1 = 4)
                result.push(0x04);

                // initialize Connect Flags
                let mut flags: u8 = 0;

                // Will Retain
                match retain {
                    Some(ref retain) => {
                        if *retain {
                            flags = flags + 0x20;
                        }
                    },
                    None => {}
                }

                // Will qos
                match qos {
                    Some(ref qos) => {
                        flags = flags + (qos << 3);
                    },
                    None => {}
                }

                // Clean Session
                if clean_session {
                    flags = flags + 0x02;
                }

                // Keep Alive
                // TODO proper encoding for longer keep alives
                result.push(0x00);
                result.push(keep_alive as u8);

                // Client Identifier
                result.append(&mut encode_string(&client_id));

                // Will Topic
                match topic {
                    Some(ref topic) => {
                        // Will Flag
                        flags = flags + 0x04;
                        result.append(&mut encode_string(&topic));
                    },
                    None => {}
                }

                // Will Message
                match content {
                    Some(ref content) => {
                        result.append(&mut encode_string(&content));
                    },
                    None => {}
                }

                // User Name
                match username {
                    Some(ref username) => {
                        flags = flags + 0x80;
                        result.append(&mut encode_string(&username));
                    },
                    None => {}
                }

                // Password
                match password {
                    Some(ref password) => {
                        flags = flags + 0x40;
                        result.append(&mut encode_string(&password));
                    },
                    None => {}
                }

                // add flags to the result
                result.insert(9, flags);
            },
            CtrlPacket::SUBSCRIBE { topic_filter, qos, packet_id } => {
                // id = 8 and reserved flags = 2
                result.push(0x82);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                for i in 0..topic_filter.len() {
                    // topic name
                    result.append(&mut encode_string(&topic_filter[i]));

                    // reserved & qos
                    result.push(qos[i]);
                }
            },
            CtrlPacket::UNSUBSCRIBE { topic_filter, packet_id } => {
                // id = 8 and reserved flags = 2
                result.push(0xa2);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                for i in 0..(topic_filter.len() - 1) {
                    // topic name
                    result.append(&mut encode_string(&topic_filter[i]));
                }
            },
            CtrlPacket::PUBLISH { packet_id, topic, mut payload,
                    duplicate_delivery, qos, retain } => {

                // id = 3
                // DUP flag / qos / retain flags
                let mut flags = 0;
                if duplicate_delivery {
                    flags += 8;
                }
                if retain {
                    flags += 1;
                }
                flags += qos * 2;
                result.push(0x30 + flags);

                // topic name
                result.append(&mut encode_string(&topic));

                // packet identifier
                if qos != 0 {
                    match packet_id {
                        Some(packet_id_value) => {
                            result.push((packet_id_value / 256) as u8);
                            result.push(packet_id_value as u8);
                        }, None => {}
                    }
                }

                // payload
                result.append(&mut payload);
            },
            CtrlPacket::PUBACK { packet_id } => {
                // id = 4
                result.push(0x40);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);
            },
            CtrlPacket::PUBREC { packet_id } => {
                // id = 5
                result.push(0x50);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);
            },
            CtrlPacket::PUBREL { packet_id } => {
                // id = 6, reserved = 2
                result.push(0x62);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);
            },
            CtrlPacket::PUBCOMP { packet_id } => {
                // id = 7
                result.push(0x70);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);
            },
            CtrlPacket::PINGREQ => {
                // id = 12
                result.push(0xc0);
            },
            CtrlPacket::PINGRESP => {
                // id = 13
                result.push(0xd0);
            },
            CtrlPacket::DISCONNECT => {
                // id = 14
                result.push(0xe0);
            },
            _ => {}
        }

        // encode "remaining length" and insert at the second position in result vector
        insert_all(encode_remaining_length(result.len() - 1), &mut result, 1);

        result
    }

    fn from_bytes(bytes: &mut Vec<u8>) -> Option<CtrlPacket> {
        if bytes.len() > 0 {
            match bytes[0] {
                0x20 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::CONNACK {
                            session_present: bytes[2] == 1,
                            return_code: bytes[3]
                        }),
                        _ => None
                    }
                },
                0x30 ... 0x3f => {
                    let remaining_length = decode_remaining_length(bytes, 1);

                    match remaining_length {
                        Some(decoded_remaining_length) => {

                            let remaining_length_length: i8 =
                                    remaining_length_length(decoded_remaining_length);

                            if decoded_remaining_length + 1 + remaining_length_length as i32
                                    == bytes.len() as i32 {

                                let duplicate_delivery: bool = bytes[0] & 0x08 > 0;
                                let qos: u8 = (bytes[0] & 0x06) / 2;
                                let retain: bool = bytes[0] & 0x01 > 0;

                                // remove the packet type and remaining length bytes
                                bytes.drain(0..(1 + remaining_length_length as usize));

                                // parse topic
                                let topic_length: usize =
                                        decode_string_length(&bytes.drain(0..2).collect());
                                let topic: String =
                                        String::from_utf8(bytes.drain(0..topic_length).collect())
                                            .unwrap();

                                // parse packet id
                                let mut packet_id = None;
                                if qos > 0 {
                                    packet_id = Some(parse_packet_id(&bytes.drain(0..2).collect()));
                                }

                                // parse payload
                                let payload = bytes.drain(..).collect();

                                Some(CtrlPacket::PUBLISH {
                                    packet_id: packet_id,
                                    topic: topic,
                                    payload: payload,
                                    duplicate_delivery: duplicate_delivery,
                                    qos: qos,
                                    retain: retain
                                })
                            } else {
                                None
                            }
                        },
                        None => None
                    }
                },
                0x40 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::PUBACK {
                                packet_id: bytes[2] as i16 * 256 + bytes[3] as i16
                            }),
                        _ => None
                    }
                },
                0x50 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::PUBREC {
                                packet_id: bytes[2] as i16 * 256 + bytes[3] as i16
                            }),
                        _ => None
                    }
                },
                0x62 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::PUBREL {
                                packet_id: bytes[2] as i16 * 256 + bytes[3] as i16
                            }),
                        _ => None
                    }
                },
                0x70 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::PUBCOMP {
                                packet_id: bytes[2] as i16 * 256 + bytes[3] as i16
                            }),
                        _ => None
                    }
                },
                0x90 => {
                    if bytes.len() > 4 {
                        Some(CtrlPacket::SUBACK {
                            packet_id: bytes[2] as i16 * 256 + bytes[3] as i16,
                            return_code: bytes[4]
                        })
                    } else {
                        None
                    }
                },
                0xb0 => {
                    if bytes.len() > 3 {
                        Some(CtrlPacket::UNSUBACK {
                            packet_id: bytes[2] as i16 * 256 + bytes[3] as i16
                        })
                    } else {
                        None
                    }
                },
                0xc0 => {
                    if bytes.len() > 1 {
                        Some(CtrlPacket::PINGREQ)
                    } else {
                        None
                    }
                },
                0xd0 => {
                    if bytes.len() > 1 {
                        Some(CtrlPacket::PINGRESP)
                    } else {
                        None
                    }
                },
                0xe0 => {
                    if bytes.len() > 1 {
                        Some(CtrlPacket::DISCONNECT)
                    } else {
                        None
                    }
                }
                _ => None
            }
        } else {
            None
        }
    }
}

// pub fn parse(ctrl_packet_as_bytes: &Vec<u8>) -> Option<Box<CtrlPacket+'static>> {
pub fn parse(ctrl_packet_as_bytes: &mut Vec<u8>) -> Option<CtrlPacket> {
    CtrlPacket::from_bytes(ctrl_packet_as_bytes)
}

fn parse_packet_id(packet_id_as_bytes: &Vec<u8>) -> i16 {
    packet_id_as_bytes[0] as i16 * 256 + packet_id_as_bytes[1] as i16
}

#[allow(unused_assignments)]
fn encode_remaining_length(input_length: usize) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    let mut tmp: i16 = 0;
    let mut length: i16 = input_length as i16;

    loop {
        tmp = length % 0x80;
        length = length / 0x80;
        if length > 0 {
            tmp = tmp | 0x80;
        }
        result.push(tmp as u8);
        if length <= 0 {
            break;
        }
    }
    result
}

// TODO return the number of bytes that are used to encode the "remaining length"
#[allow(unused_assignments)]
fn decode_remaining_length(remaining_length: &Vec<u8>, offset: i8) -> Option<i32> {
    let mut multiplier: i32 = 1;
    let mut value: i32 = 0;
    let mut encoded_byte: u8 = 0;
    let mut counter: i8 = offset;

    if remaining_length.len() as i8 - offset > 0 {
        loop {
            encoded_byte = remaining_length[counter as usize];
            value += (encoded_byte & 127) as i32 * multiplier;

            multiplier *= 128;
            if multiplier > 128 * 128 * 128 {
                // Malformed Remaining Length (not complete?)
                return None
            }
            if (encoded_byte & 128) == 0 {
                break;
            }
            counter += 1;
            if counter == remaining_length.len() as i8 - offset {
                return None
            }
        }
    }
    Some(value)
}

fn remaining_length_length(remaining_length: i32) -> i8 {
    match remaining_length {
        0...127 => 1,
        128...16383 => 2,
        16384...2097151 => 3,
        _ => 4
    }
}

fn encode_string(string_to_encode: &str) -> Vec<u8> {
    let string_length: usize = string_to_encode.len();
    let mut result: Vec<u8> = Vec::new();

    let tmp: u16 = string_length as u16;

    // encode the length of the string
    result.push((tmp / 256) as u8);
    result.push((tmp % 256) as u8);

    // write the string to the result-array
    let string_as_bytes = string_to_encode.as_bytes();
    for i in 0 .. string_length {
        result.push(string_as_bytes[i]);
    }
    result
}

fn decode_string_length(bytes: &Vec<u8>) -> usize {
    let mut decoded_string_length: usize = bytes[0] as usize * 256 as usize;
    decoded_string_length += bytes[1] as usize;
    decoded_string_length
}

fn insert_all(source: Vec<u8>, target: &mut Vec<u8>, index: usize) {
    for i in (0..source.len()).rev() {
        target.insert(index, source[i]);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use super::encode_string;
    use super::encode_remaining_length;
    use super::decode_remaining_length;
    use super::decode_string_length;
    use super::insert_all;

    #[test]
    fn test_encode_string1() {
        let result: Vec<u8> = encode_string("TESTT");
        assert_eq!(0x00, result[0]);
        assert_eq!(0x05, result[1]);
        assert_eq!(0x54, result[2]);
        assert_eq!(0x45, result[3]);
        assert_eq!(0x53, result[4]);
        assert_eq!(0x54, result[5]);
        assert_eq!(0x54, result[6]);
    }

    #[test]
    fn test_encode_string2() {
        let result: Vec<u8> = encode_string("MQTT");
        assert_eq!(0x00, result[0]);
        assert_eq!(0x04, result[1]);
        assert_eq!(0x4d, result[2]);
        assert_eq!(0x51, result[3]);
        assert_eq!(0x54, result[4]);
        assert_eq!(0x54, result[5]);
    }

    #[test]
    fn test_encode_remaining_length1() {
        let result: Vec<u8> = encode_remaining_length(20);
        assert_eq!(1, result.len());
        assert_eq!(0x14, result[0]);
    }

    #[test]
    fn test_encode_remaining_length2() {
        let result: Vec<u8> = encode_remaining_length(1307);
        assert_eq!(2, result.len());
        assert_eq!(0x9b, result[0]);
        assert_eq!(0x0a, result[1]);
    }

    #[test]
    fn test_encode_remaining_length3() {
        let result: Vec<u8> = encode_remaining_length(16387);
        assert_eq!(3, result.len());
        assert_eq!(0x83, result[0]);
        assert_eq!(0x80, result[1]);
        assert_eq!(0x01, result[2]);
    }

    #[test]
    fn test_decode_remaining_length1() {
        let encoded_remaining_length: Vec<u8> = encode_remaining_length(20);
        assert_eq!(20, decode_remaining_length(&encoded_remaining_length, 0).unwrap());
    }

    #[test]
    fn test_decode_remaining_length2() {
        let encoded_remaining_length: Vec<u8> = encode_remaining_length(1307);
        assert_eq!(1307, decode_remaining_length(&encoded_remaining_length, 0).unwrap());
    }

    #[test]
    fn test_decode_remaining_length3() {
        let encoded_remaining_length: Vec<u8> = encode_remaining_length(16387);
        assert_eq!(16387, decode_remaining_length(&encoded_remaining_length, 0).unwrap());
    }

    #[test]
    fn test_insert_all() {
        let mut v1 = vec![0x01, 0x02, 0x03, 0x04];
        let mut v2 = vec![0x05, 0x06, 0x07];

        insert_all(v2, &mut v1, 2);
        assert_eq!(v1, [0x01, 0x02, 0x05, 0x06, 0x07, 0x03, 0x04]);
    }
}
