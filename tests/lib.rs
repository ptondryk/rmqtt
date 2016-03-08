extern crate rmqtt;

use rmqtt::MqttSessionBuilder;

#[test]
fn test_connection_refused() {
    match MqttSessionBuilder::new("test-client-01", "localhost:1883")
            .credentials("user", "password")
            .keep_alive(120)
            .connect() {
        Err(message) => {
            assert_eq!("Connection refused (os error 61)", message);
        },
        _ => {
            // Err should occure
            assert!(false);
        }
    }
}
