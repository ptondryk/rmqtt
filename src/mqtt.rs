pub struct CONNECT {
    clientId: String,
    topic: Option<String>,
    content: Option<String>,
    QoS: Option<u8>,
    retain: Option<bool>,
    username: Option<String>,
    password: Option<String>,
    clean_session: bool,
    keep_alive: i16
}

struct CONNACK;

struct PUBLISH {
    duplicateDelivery: bool,
    QoS: u8,
    retain: bool
}

struct PUBACK;
struct PUBREC;
struct PUBREL;
struct PUBCOMP;
struct SUBSCRIBE;
struct SUBACK;
struct UNSUBSCRIBE;
struct UNSUBACK;
struct PINGREQ;
struct PINGRESP;
struct DISCONNECT;

impl CONNECT {
    pub fn new(clientId: &str) -> CONNECT {
        CONNECT {
            clientId: clientId.to_string(),
            topic: None,
            content: None,
            QoS: None,
            retain: None,
            username: None,
            password: None,
            clean_session: false,
            keep_alive: 10
        }
    }

    pub fn new_with_authentication(clientId: &str, username: &str, password: &str) -> CONNECT {
        CONNECT {
            clientId: clientId.to_string(),
            topic: None,
            content: None,
            QoS: None,
            retain: None,
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            clean_session: false,
            keep_alive: 10
        }
    }

    pub fn new_with_message(clientId: &str, topic: &str, content: &str, QoS: u8,
            retain: bool) -> CONNECT {
        CONNECT {
            clientId: clientId.to_string(),
            topic: Some(topic.to_string()),
            content: Some(content.to_string()),
            QoS: Some(QoS),
            retain: Some(retain),
            username: None,
            password: None,
            clean_session: false,
            keep_alive: 10
        }
    }

    pub fn new_with_message_and_authntication(clientId: &str, topic: &str,
            content: &str, QoS: u8, retain: bool, username: &str, password: &str) -> CONNECT {
        CONNECT {
            clientId: clientId.to_string(),
            topic: Some(topic.to_string()),
            content: Some(content.to_string()),
            QoS: Some(QoS),
            retain: Some(retain),
            username: Some(username.to_string()),
            password: Some(password.to_string()),
            clean_session: false,
            keep_alive: 10
        }
    }
}

pub trait CtrlPacket {
    fn as_bytes(&self) -> Vec<u8>;
}

impl CtrlPacket for CONNECT {
    fn as_bytes(&self) -> Vec<u8> {
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
        match self.retain {
            Some(ref retain) => {
                if(*retain) {
                    flags = flags + 0x20;
                }
            },
            None => {}
        }

        // Will QoS
        match self.QoS {
            Some(ref QoS) => {
                flags = flags + (QoS << 3);
            },
            None => {}
        }

        // Clean Session
        if(self.clean_session) {
            flags = flags + 0x02;
        }

        // Keep Alive
        // TODO proper encoding for longer keep alives
        result.push(0x00);
        result.push(self.keep_alive as u8);

        // Client Identifier
        result.append(&mut encode_string(&self.clientId));

        // Will Topic
        match self.topic {
            Some(ref topic) => {
                // Will Flag
                flags = flags + 0x04;
                result.append(&mut encode_string(&topic));
            },
            None => {}
        }

        // Will Message
        match self.content {
            Some(ref content) => {
                result.append(&mut encode_string(&content));
            },
            None => {}
        }

        // User Name
        match self.username {
            Some(ref username) => {
                flags = flags + 0x80;
                result.append(&mut encode_string(&username));
            },
            None => {}
        }

        // Password
        match self.password {
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
    }
}

fn parse(ctrl_packet_as_string: &str) -> Option<Box<CtrlPacket+'static>> {
    // TODO implement
    None
}

fn encode_remaining_length(input_length: usize) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    let mut tmp: i16 = 0;
    let mut length: i16 = input_length as i16;

    loop {
        tmp = length % 0x80;
        length = length / 0x80;
        if (length > 0) {
            tmp = tmp | 0x80;
        }
        result.push(tmp as u8);
        if (length <= 0) {
            break;
        }
    }
    result
}

fn decode_remaining_length(remaining_length: Vec<u8>) -> i32 {
    let mut multiplier: i32 = 1;
    let mut value: i32 = 0;
    let mut encodedByte: u8 = 0;
    let mut counter: i8 = 0;

    loop {
        encodedByte = remaining_length[counter as usize];
        value += (encodedByte & 127) as i32 * multiplier;

        multiplier *= 128;
        if (multiplier > 128 * 128 * 128) {
            panic!("Malformed Remaining Length");
        }
        if ((encodedByte & 128) == 0) {
            break;
        }
        counter += 1;
    }
    value
}

fn encode_string(string_to_encode: &str) -> Vec<u8> {
    let string_length: usize = string_to_encode.len();
    let mut result: Vec<u8> = Vec::new();

    let tmp: u16 = string_length as u16;

    // encode the length of the string
    result.push((tmp / 16) as u8);
    result.push((tmp % 16) as u8);

    // write the string to the result-array
    let string_as_bytes = string_to_encode.as_bytes();
    for i in 0 .. string_length {
        result.push(string_as_bytes[i]);
    }
    result
}

fn insert_all(source: Vec<u8>, target: &mut Vec<u8>, index: usize) {
    for i in (0..source.len()).rev() {
        target.insert(index, source[i]);
    }
}

#[test]
fn test_encode_string1() {
    let result: Vec<u8> = encode_string("TEST");
    assert_eq!(0x00, result[0]);
    assert_eq!(0x04, result[1]);
    assert_eq!(0x54, result[2]);
    assert_eq!(0x45, result[3]);
    assert_eq!(0x53, result[4]);
    assert_eq!(0x54, result[5]);
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
    assert_eq!(20, decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_decode_remaining_length2() {
    let encoded_remaining_length: Vec<u8> = encode_remaining_length(1307);
    assert_eq!(1307, decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_decode_remaining_length3() {
    let encoded_remaining_length: Vec<u8> = encode_remaining_length(16387);
    assert_eq!(16387, decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_insert_all() {
    let mut v1 = vec![0x01, 0x02, 0x03, 0x04];
    let mut v2 = vec![0x05, 0x06, 0x07];

    insert_all(v2, &mut v1, 2);
    assert_eq!(v1, [0x01, 0x02, 0x05, 0x06, 0x07, 0x03, 0x04]);
}
