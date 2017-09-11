extern crate rumqtt;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::sync::{mpsc, Mutex};
use rumqtt::{MqttCallback, MqttClient, MqttOptions, QoS};

const MQTT_HOST: &str = "localhost:1883";
const ERR_TOPIC: &str = "errors";
const IRC_TOPIC: &str = "actors/all/flipbot_send";

#[derive(Serialize, Deserialize)]
struct ErrorMessage {
    origin: String,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct IrcMessage {
    content: String,
}

fn main() {
    // TODO: command line args

    let client_options = MqttOptions::new()
        .set_keep_alive(5)
        .set_reconnect(3)
        .set_broker(MQTT_HOST);

    let (tx, rx) = mpsc::channel::<ErrorMessage>();
    let tx = Mutex::new(tx);

    let mq_cbs = MqttCallback::new().on_message(move |msg| {
        if msg.topic.as_str() != ERR_TOPIC {
            // TODO: log error
            return;
        }

        match serde_json::from_slice(&msg.payload) {
            Ok(err_msg) => tx.lock().unwrap().send(err_msg).unwrap(),
            Err(_) => {
                // TODO: log error
            }
        }
    });

    let mut client = MqttClient::start(client_options, Some(mq_cbs)).expect("Coudn't start");
    client.subscribe(vec![(ERR_TOPIC, QoS::Level0)]).unwrap();

    loop {
        let err_msg = rx.recv().unwrap();
        let irc_msg = IrcMessage {
            content: format!("Error in {}: {}", err_msg.origin, err_msg.message),
        };

        client
            .publish(
                IRC_TOPIC,
                QoS::Level0,
                serde_json::to_vec(&irc_msg).unwrap(),
            )
            .unwrap();
    }
}
