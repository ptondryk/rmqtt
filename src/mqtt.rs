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

        // TODO calculate, encode and add the "remaining length" to the result
        // TODO now it is hard-coded length
        result.push(0x29);

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
        result.append(&mut encode_string("testClientId"));

        // Will Topic
        // result.append(&mut encode_string("testTopic"));

        // Will Message
        // result.append(&mut encode_string("testMessageContent"));

        // User Name
        result.append(&mut encode_string("system"));

        // Password
        result.append(&mut encode_string("manager"));

        result
    }
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

#[test]
fn test_encode_string() {
    let result: Vec<u8> = encode_string("TEST");
    assert_eq!(0x00, result[0]);
    assert_eq!(0x04, result[1]);
    assert_eq!(0x54, result[2]);
    assert_eq!(0x45, result[3]);
    assert_eq!(0x53, result[4]);
    assert_eq!(0x54, result[5]);
}
