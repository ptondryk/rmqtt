extern crate rmqtt;

use rmqtt::MqttSessionBuilder;
use rmqtt::ConnectFailed;

#[test]
#[allow(unused_variables)]
fn test_connection_refused() {
    match MqttSessionBuilder::new("test-client-01", "localhost:1884")
            .credentials("user", "password")
            .keep_alive(120)
            .connect() {
        Err(error_type) => {
			match error_type {
				ConnectFailed::ConnectionError { details } => {},
				_ => { assert!(false); }
			}
        },
        _ => {
            // Err should occure
            assert!(false);
        }
    }
}

// TODO write more tests
