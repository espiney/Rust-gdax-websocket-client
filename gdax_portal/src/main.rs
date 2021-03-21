extern crate websocket;
extern crate redis;

use std::io::stdin;
use std::sync::mpsc::channel;
use std::thread;
//use std::collections::HashMap;

use websocket::client::ClientBuilder;
use websocket::{Message, OwnedMessage};

const CONNECTION: &'static str = "ws://127.0.0.1";

fn main() {
    println!("WSB MAIN CONNECTING {}", CONNECTION);

    let client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    let mut _counter = 0;

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
        let subscribe_text = "{\"type\": \"subscribe\", \"channels\": [{\"name\": \"full\", \"product_ids\": [\"MATIC-BTC\", \"SUSHI-EUR\", \"SUSHI-GBP\", \"MATIC-GBP\", \"MATIC-USD\", \"MATIC-EUR\", \"SKL-GBP\", \"SKL-USD\", \"SKL-BTC\", \"SKL-EUR\", \"ADA-USD\", \"ADA-BTC\", \"ADA-EUR\", \"ADA-GBP\", \"SUSHI-BTC\", \"SUSHI-ETH\", \"SUSHI-USD\", \"AAVE-USD\", \"ALGO-BTC\", \"BNT-BTC\", \"BNT-USD\", \"CGLD-EUR\", \"COMP-BTC\", \"LTC-BTC\", \"ETC-BTC\", \"ETH-BTC\", \"LINK-ETH\", \"CVC-USDC\", \"DNT-USDC\", \"LOOM-USDC\", \"LINK-GBP\", \"AAVE-BTC\", \"AAVE-EUR\", \"BTC-USD\", \"AAVE-GBP\", \"BAT-ETH\", \"BNT-EUR\", \"BNT-GBP\", \"DAI-USD\", \"EOS-BTC\", \"FIL-BTC\", \"FIL-EUR\", \"FIL-GBP\", \"FIL-USD\", \"GRT-BTC\", \"GRT-EUR\", \"GRT-GBP\", \"GRT-USD\", \"KNC-USD\", \"LINK-BTC\", \"LRC-BTC\", \"LRC-USD\", \"ALGO-GBP\", \"LTC-EUR\", \"BAL-USD\", \"BAND-USD\", \"BAND-BTC\", \"BAND-EUR\", \"BAND-GBP\", \"CGLD-BTC\", \"CGLD-USD\", \"MKR-BTC\", \"MKR-USD\", \"NMR-USD\", \"NMR-BTC\", \"NMR-EUR\", \"NMR-GBP\", \"NU-BTC\", \"NU-EUR\", \"NU-GBP\", \"NU-USD\", \"OMG-BTC\", \"OMG-EUR\", \"OMG-GBP\", \"REN-BTC\", \"REN-USD\", \"REP-BTC\", \"REP-USD\", \"SNX-BTC\", \"SNX-EUR\", \"SNX-GBP\", \"SNX-USD\", \"UMA-BTC\", \"UMA-EUR\", \"UMA-GBP\", \"LTC-GBP\", \"UNI-BTC\", \"LTC-USD\", \"ETC-EUR\", \"ETC-GBP\", \"ETC-USD\", \"ALGO-USD\", \"BAT-USDC\", \"ETH-GBP\", \"ETH-USDC\", \"BCH-BTC\", \"BCH-EUR\", \"BCH-GBP\", \"BCH-USD\", \"ETH-USD\", \"UNI-USD\", \"LINK-EUR\", \"BTC-EUR\", \"EOS-USD\", \"BTC-USDC\", \"BTC-GBP\", \"KNC-BTC\", \"OMG-USD\", \"UMA-USD\", \"WBTC-BTC\", \"WBTC-USD\", \"XLM-BTC\", \"XLM-EUR\", \"ETH-EUR\", \"ETH-DAI\", \"GNT-USDC\", \"MANA-USDC\", \"LINK-USD\", \"ALGO-EUR\", \"ATOM-BTC\", \"ATOM-USD\", \"BAL-BTC\", \"CGLD-GBP\", \"COMP-USD\", \"DAI-USDC\", \"DASH-BTC\", \"DASH-USD\", \"EOS-EUR\", \"OXT-USD\", \"XLM-USD\", \"XTZ-USD\", \"XTZ-EUR\", \"XTZ-GBP\", \"XTZ-BTC\", \"YFI-BTC\", \"YFI-USD\", \"ZEC-USD\", \"ZEC-BTC\", \"ZEC-USDC\", \"ZRX-BTC\", \"ZRX-EUR\", \"ZRX-USD\"]}]}".to_string();
        tx_1.send(OwnedMessage::Text(subscribe_text));

        // Receive loop
        for message in receiver.incoming_messages() {
            // Updated the general RX Counter
            _counter += 1;

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
                    println!("WSB RX ERROR STREAM Unhandled packet {:?}, {:?}",message,_counter);
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
                // let mut packet_hash = HashMap::new();

                // Split on ',' as we should have no nested classes
                // let message_split = message_clean.split(",");

                // Save all the fragments within a hash
                // for item in message_split {
                //     let keyval: String = item.to_string();
                //     let (keyval_lhs,keyval_rhs) = item.split_at(keyval.find(":").unwrap());
                //     packet_hash.insert(keyval_lhs,keyval_rhs.trim_start_matches(":"));
                // }

                println!("WSB RX DATA {}",message_clean);
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
                println!("WSB MAIN ERROR {:?}", e);
                break;
            }
        }
    }

    // We're exiting

    println!("WSB MAIN EXITING");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("WSB MAIN EXIT");
}
