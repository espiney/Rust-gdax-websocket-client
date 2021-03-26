extern crate websocket;
extern crate redis;

//use std::io::stdin;
use std::sync::mpsc::channel;
use std::fs;
use std::thread;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

//use redis::{PubSubCommands, ControlFlow};

const CONNECTION: &'static str = "ws://127.0.0.1";

fn connect() -> redis::Connection {
    let redis_conn_url = "redis+unix:///tmp/redis.sock";
    redis::Client::open(redis_conn_url)
        .expect("Invalid connection URL")
        .get_connection()
        .expect("failed to connect to Redis")
}


fn main() {
    println!("WSB MAIN CONNECTING {}", CONNECTION);

    // Multiple connections to redis ...
    let mut conn_set = connect();
    let mut conn_publish = connect();

    let client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    println!("WSB MAIN CONNECT");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("WSB TX ERROR MAIN {:?}", e);
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&message);
                    // If it's a close message, just send it and then return.
                    return;
                },
                _ => (),
            }
            // Send the message
            match sender.send_message(&message) {
                Ok(()) => (),
                Err(e) => {
                    println!("WSB TX ERROR SEND {:?}", e);
                    let _ = sender.send_message(&Message::close());
                    return;
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        // Send subscribe packet
        let subscribe_text = fs::read_to_string("/tmp/subscription.json").expect("Something went wrong reading the file");
        tx_1.send(OwnedMessage::Text(subscribe_text));

        // Receive loop
        for message in receiver.incoming_messages() {
            // Match specific events in the websocket stream
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    println!("WSB RX ERROR MAIN {:?}", e);
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
            };
            let message = match message {
                OwnedMessage::Close(_) => {
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => {
                            return;
                        },
                        Err(e) => {
                            println!("WSB RX ERROR PING {:?}", e);
                            return;
                        }
                    }
                }
                OwnedMessage::Text(content) => { 
                    content
                }
                _ => {
                    println!("WSB RX ERROR STREAM Unhandled packet {:?}",message);
                    return;
                }
            };

            // Attempt to extract type
            let (_,type_rhs) =
                message.split_at(message.find("type").unwrap()+7);
            let (message_type,_) =
                type_rhs.split_at(type_rhs.find('"').unwrap());

            // If we are a subscriprion, then simply return we do not care
            if message_type != "subscriptions" { 
                let (_,product_rhs) =
                    message.split_at(message.find("product_id").unwrap()+13);
                let (product_id,_) =
                    product_rhs.split_at(product_rhs.find('"').unwrap());

                let (_,sequence) =
                    message.split_at(message.find("sequence").unwrap()+10);
                let (message_sequence,_) =
                    sequence.split_at(sequence.find(',').unwrap());

                // Add a pubsub for the currency if one does not exist
                let pkey = format!("{}:{}",product_id,message_sequence);

                // Set it in the main set
                let _: () = redis::cmd("SET").arg(pkey).arg(&message).query(&mut conn_set).expect("");
                let _: () = redis::cmd("PUBLISH").arg(product_id).arg(message_sequence).query(&mut conn_publish).expect("");

                //pubsub.subscribe(product_id);
            }
        }
    });

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("WSB MAIN EXIT");
}
