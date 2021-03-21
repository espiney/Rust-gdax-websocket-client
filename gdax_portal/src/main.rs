extern crate websocket;
extern crate redis;

use std::io::stdin;
use std::sync::mpsc::channel;
use std::thread;
use std::collections::HashMap;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

use redis::Commands;

const CONNECTION: &'static str = "ws://127.0.0.1";

fn main() {
    println!("Connecting to {}", CONNECTION);

    let client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    let redis_client = redis::Client::open("redis://127.0.0.1/");
    let mut _counter = 0;

    println!("Successfully connected");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("Send Loop: {:?}", e);
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
                    println!("Send Loop: {:?}", e);
                    let _ = sender.send_message(&Message::close());
                    return;
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        // Receive loop
        for message in receiver.incoming_messages() {
            // Updated the general RX Counter
            _counter += 1;

            // Match specific events in the websocket stream
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    println!("Receive Loop: {:?}", e);
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
                            println!("Receive Loop: {:?}", e);
                            return;
                        }
                    }
                }
                OwnedMessage::Text(content) => { 
                    content
                }
                _ => {
                    println!("RXC({:?} Unhandled packet: {:?}",_counter,message);
                    return;
                }
            };

            // Lets digest this json packet
            let message_clean = message
                .trim_end_matches("}")
                .trim_start_matches("{")
                .replace("\n","")
                .replace("\r","")
                .replace("\"","");

            // Attempt to extract type
            let (_,type_rhs) =
                message_clean.split_at(message_clean.find("type").unwrap()+5);
            let (message_type,_) =
                type_rhs.split_at(message_clean.find(",").unwrap()-5);

            // If we are a subscriprion, then simply return we do not care
            if message_type != "subscriptions" {
                // If we got here we have a true string!
                let mut packet_hash = HashMap::new();

                // Split on ',' as we should have no nested classes
                let message_split = message_clean.split(",");

                // Save all the fragments within a hash
                for item in message_split {
                    let keyval: String = item.to_string();
                    let (keyval_lhs,keyval_rhs) = item.split_at(keyval.find(":").unwrap());
                    packet_hash.insert(keyval_lhs,keyval_rhs.trim_start_matches(":"));
                }

                // let redis_pkey = format!("{}{}",packet_hash["product_id"],packet_hash["sequence"]);
                // redis::cmd("SET").arg(redis_pkey).arg(message).query(redis_client);

                println!("{}:{}",packet_hash["product_id"],packet_hash["sequence"]);
            }
        }
    });

    loop {
        let mut input = String::new();

        stdin().read_line(&mut input).unwrap();

        let trimmed = input.trim();

        let message = match trimmed {
            "/close" => {
                // Close the connection
                let _ = tx.send(OwnedMessage::Close(None));
                break;
            }
            // Send a ping
            "/ping" => OwnedMessage::Ping(b"PING".to_vec()),
            // Otherwise, just send text
            _ => OwnedMessage::Text(trimmed.to_string()),
        };

        match tx.send(message) {
            Ok(()) => (),
            Err(e) => {
                println!("Main Loop: {:?}", e);
                break;
            }
        }
    }

    // We're exiting

    println!("Waiting for child threads to exit");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("Exited");
}