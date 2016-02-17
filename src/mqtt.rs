use std::fmt;

#[derive(Debug)]
pub enum CtrlPacket {
    CONNECT {
        clientId: String,
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
        connect_return_code: u8
    },
    PUBLISH {
        packet_id: i16,
        topic: String,
        payload: String,
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

    pub fn new_publish(topic: &str, payload: &str, packet_id: i16) -> CtrlPacket {
        CtrlPacket::PUBLISH {
            packet_id: packet_id,
            topic: topic.to_string(),
            payload: payload.to_string(),
            // TODO set the values properly
            duplicate_delivery: false,
            qos: 0x00,
            retain: false
        }
    }

    pub fn as_bytes(self) -> Vec<u8> {
        match self {
            CtrlPacket::CONNECT { clientId, topic, content, qos, retain, username,
                        password, clean_session, keep_alive } => {
                let mut result: Vec<u8> = Vec::new();

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
                result.append(&mut encode_string(&clientId));

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

                // encode "remaining length" and insert at the second position in result vector
                insert_all(encode_remaining_length(result.len() - 1), &mut result, 1);

                result
            },
            CtrlPacket::SUBSCRIBE { topic_filter, qos, packet_id } => {
                let mut result: Vec<u8> = Vec::new();

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

                // encode "remaining length" and insert at the second position in result vector
                insert_all(encode_remaining_length(result.len() - 1), &mut result, 1);

                result
            },
            CtrlPacket::UNSUBSCRIBE { topic_filter, packet_id } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 8 and reserved flags = 2
                result.push(0xa2);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                for i in 0..(topic_filter.len() - 1) {
                    // topic name
                    result.append(&mut encode_string(&topic_filter[i]));
                }

                // encode "remaining length" and insert at the second position in result vector
                insert_all(encode_remaining_length(result.len() - 1), &mut result, 1);

                result
            },
            CtrlPacket::PUBLISH { packet_id, topic, payload, duplicate_delivery, qos, retain } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 3
                // TODO set DUP flag / qos / retain flag properly
                result.push(0x30);

                // topic name
                result.append(&mut encode_string(&topic));

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                // payload
                result.append(&mut array_to_vec(payload.as_bytes()));

                // encode "remaining length" and insert at the second position in result vector
                insert_all(encode_remaining_length(result.len() - 1), &mut result, 1);

                result
            },
            CtrlPacket::PUBACK { packet_id } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 4
                result.push(0x40);

                // remaining length, always = 2
                result.push(0x02);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                result
            },
            CtrlPacket::PUBREC { packet_id } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 5
                result.push(0x50);

                // remaining length, always = 2
                result.push(0x02);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                result
            },
            CtrlPacket::PUBREL { packet_id } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 6
                result.push(0x60);

                // remaining length, always = 2
                result.push(0x02);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                result
            },
            CtrlPacket::PUBCOMP { packet_id } => {
                let mut result: Vec<u8> = Vec::new();

                // id = 7
                result.push(0x70);

                // remaining length, always = 2
                result.push(0x02);

                // packet identifier
                result.push((packet_id / 256) as u8);
                result.push(packet_id as u8);

                result
            },
            CtrlPacket::PINGREQ => {
                let mut result: Vec<u8> = Vec::new();

                // id = 12
                result.push(0xc0);

                // remaining length, always = 0
                result.push(0x00);

                result
            },
            CtrlPacket::PINGRESP => {
                let mut result: Vec<u8> = Vec::new();

                // id = 13
                result.push(0xd0);

                // remaining length, always = 0
                result.push(0x00);

                result
            },
            CtrlPacket::DISCONNECT => {
                let mut result: Vec<u8> = Vec::new();

                // id = 14
                result.push(0xe0);

                // remaining length, always = 0
                result.push(0x00);

                result
            },
            _ => {
                // TODO implement
                Vec::new()
            }
        }
    }

    fn from_bytes(bytes: &Vec<u8>) -> Option<CtrlPacket> {
        if bytes.len() > 0 {
            match bytes[0] {
                0x20 => {
                    match bytes.len() {
                        4 => Some(CtrlPacket::CONNACK {
                            session_present: bytes[2] == 1,
                            connect_return_code: bytes[3]
                        }),
                        _ => None
                    }
                },
                0x30 => {
                    let remaining_length = decode_remaining_length(bytes, 1);

                    match remaining_length {
                        Some(decoded_remaining_length) => {
                            if decoded_remaining_length + 1 + remaining_length_length(decoded_remaining_length) as i32
                                    == bytes.len() as i32 {
                                // TODO optimize
                                let (_, variable_header_and_payload)
                                    = bytes.split_at((1 + remaining_length_length(decoded_remaining_length)) as usize);
                                let topic_length = decode_string_length(&array_to_vec(variable_header_and_payload), 0);
                                let (encoded_topic, _) = variable_header_and_payload.split_at((topic_length + 2) as usize);
                                let (_, topic) = encoded_topic.split_at(2);
                                let topic_as_string: String = String::from_utf8(array_to_vec(topic)).unwrap();
                                let packet_id = (variable_header_and_payload[topic_length as usize] * 256) as i16 +
                                    variable_header_and_payload[(topic_length + 1) as usize] as i16;
                                let(_, payload) = variable_header_and_payload.split_at((topic_length + 2) as usize);
                                let payload_as_string = String::from_utf8(array_to_vec(payload)).unwrap();

                                // TODO parse all parameters properly
                                Some(CtrlPacket::PUBLISH {
                                    packet_id: packet_id,
                                    topic: topic_as_string,
                                    payload: payload_as_string,
                                    duplicate_delivery: false,
                                    qos: 0,
                                    retain: false
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
                0x60 => {
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
pub fn parse(ctrl_packet_as_bytes: &Vec<u8>) -> Option<CtrlPacket> {
    CtrlPacket::from_bytes(ctrl_packet_as_bytes)
}

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

fn decode_remaining_length(remaining_length: &Vec<u8>, offset: i8) -> Option<i32> {
    let mut multiplier: i32 = 1;
    let mut value: i32 = 0;
    let mut encodedByte: u8 = 0;
    let mut counter: i8 = offset;

    if remaining_length.len() as i8 - offset > 0 {
        loop {
            encodedByte = remaining_length[counter as usize];
            value += (encodedByte & 127) as i32 * multiplier;

            multiplier *= 128;
            if multiplier > 128 * 128 * 128 {
                // Malformed Remaining Length (not complete?)
                return None
            }
            if (encodedByte & 128) == 0 {
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

fn remaining_length_length(remaining_length: i32) -> i16 {
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

fn decode_string_length(bytes: &Vec<u8>, offset: i8) -> i16 {
    let mut decoded_string_length: i16 = bytes[offset as usize] as i16 * 256 as i16;
    decoded_string_length += bytes[offset as usize + 1] as i16;
    decoded_string_length
}

fn insert_all(source: Vec<u8>, target: &mut Vec<u8>, index: usize) {
    for i in (0..source.len()).rev() {
        target.insert(index, source[i]);
    }
}

fn array_to_vec(arr: &[u8]) -> Vec<u8> {
     arr.iter().cloned().collect()
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
