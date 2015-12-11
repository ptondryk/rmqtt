pub enum CtrlPacket {
    CONNECT,
    CONNACK,
    PUBLISH { duplicateDelivery: bool, QoS: u8, retain: bool },
    PUBACK,
    PUBREC,
    PUBREL,
    PUBCOMP,
    SUBSCRIBE,
    SUBACK,
    UNSUBSCRIBE,
    UNSUBACK,
    PINGREQ,
    PINGRESP,
    DISCONNECT
}

impl CtrlPacket {

    pub fn parse(ctrl_packet_as_string: &str) -> Option<CtrlPacket> {
        // TODO implement
        None
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match *self {
            CtrlPacket::CONNECT => self.connect_as_bytes(),
            _ => unimplemented!()
        }
    }

    fn connect_as_bytes(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();

        // id = 1 and reserved flags = 0
        result.push(0x10);

        // Protocol Name
        // "MQTT" encoded as specified in (1.5.3)
        result.push(0x00);
        result.push(0x04);
        result.push(0x4d);
        result.push(0x51);
        result.push(0x54);
        result.push(0x54);

        // Protocol Level (MQTT 3.1.1 = 4)
        result.push(0x04);

        // initialize Connect Flags
        let mut flags: u8 = 0;

        // User Name Flag
        flags = flags + 0x80;

        // Password Flag
        flags = flags + 0x40;

        // Will Retain
        // flags = flags + 0x20;

        // Will QoS
        flags = flags + 0x08;

        // Will Flag
        // flags = flags + 0x04;

        // Clean Session
        flags = flags + 0x02;

        // add flags to result
        result.push(flags);

        // Keep Alive (10 seconds)
        result.push(0x00);
        result.push(0x0a);

        // Client Identifier
        result.append(&mut CtrlPacket::encode_string("testClientId"));

        // Will Topic
        // result.append(&mut encode_string("testTopic"));

        // Will Message
        // result.append(&mut encode_string("testMessageContent"));

        // User Name
        result.append(&mut CtrlPacket::encode_string("system"));

        // Password
        result.append(&mut CtrlPacket::encode_string("manager"));

        // encode "remaining length" and insert at the second position in result vector
        CtrlPacket::insert_all(CtrlPacket::encode_remaining_length(result.len() - 1), &mut result, 1);

        result
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
}

#[test]
fn test_encode_string() {
    let result: Vec<u8> = CtrlPacket::encode_string("TEST");
    assert_eq!(0x00, result[0]);
    assert_eq!(0x04, result[1]);
    assert_eq!(0x54, result[2]);
    assert_eq!(0x45, result[3]);
    assert_eq!(0x53, result[4]);
    assert_eq!(0x54, result[5]);
}

#[test]
fn test_encode_remaining_length1() {
    let result: Vec<u8> = CtrlPacket::encode_remaining_length(20);
    assert_eq!(1, result.len());
    assert_eq!(0x14, result[0]);
}

#[test]
fn test_encode_remaining_length2() {
    let result: Vec<u8> = CtrlPacket::encode_remaining_length(1307);
    assert_eq!(2, result.len());
    assert_eq!(0x9b, result[0]);
    assert_eq!(0x0a, result[1]);
}

#[test]
fn test_encode_remaining_length3() {
    let result: Vec<u8> = CtrlPacket::encode_remaining_length(16387);
    assert_eq!(3, result.len());
    assert_eq!(0x83, result[0]);
    assert_eq!(0x80, result[1]);
    assert_eq!(0x01, result[2]);
}

#[test]
fn test_decode_remaining_length1() {
    let encoded_remaining_length: Vec<u8> = CtrlPacket::encode_remaining_length(20);
    assert_eq!(20, CtrlPacket::decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_decode_remaining_length2() {
    let encoded_remaining_length: Vec<u8> = CtrlPacket::encode_remaining_length(1307);
    assert_eq!(1307, CtrlPacket::decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_decode_remaining_length3() {
    let encoded_remaining_length: Vec<u8> = CtrlPacket::encode_remaining_length(16387);
    assert_eq!(16387, CtrlPacket::decode_remaining_length(encoded_remaining_length));
}

#[test]
fn test_insert_all() {
    let mut v1 = vec![0x01, 0x02, 0x03, 0x04];
    let mut v2 = vec![0x05, 0x06, 0x07];

    CtrlPacket::insert_all(v2, &mut v1, 2);

    assert_eq!(v1, [0x01, 0x02, 0x05, 0x06, 0x07, 0x03, 0x04]);
}
