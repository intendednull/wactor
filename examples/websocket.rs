use lunatic::net::{TcpListener, TcpStream};
use serde::{Deserialize, Serialize};
use tungstenite::server;
use wactor::*;

#[derive(Serialize, Deserialize, Debug)]
enum Message {
    Ping,
    Pong,
}

struct Server;
impl Actor for Server {
    type Input = TcpStream;
    type Output = ();

    fn create() -> Self {
        Self
    }

    fn handle(&mut self, stream: Self::Input, link: &Link<Self>) {
        let mut ws = server::accept(stream).unwrap();
        loop {
            let msg = ws.read_message();
            match msg {
                Ok(tungstenite::Message::Binary(msg)) => {
                    let msg = bincode::deserialize::<Message>(&msg).unwrap();

                    let response = match msg {
                        Message::Ping => Message::Pong,
                        Message::Pong => Message::Ping,
                    };

                    let buf = bincode::serialize(&response).unwrap();
                    ws.write_message(buf.into()).unwrap();

                    // Exit loop when after a successful response
                    break;
                }
                Err(_) => break,
                _ => {}
            }
        }
        // Signal this is actor is ready to be dropped.
        link.close();
    }
}

struct Listener;
impl Actor for Listener {
    type Input = String;
    type Output = ();

    fn create() -> Self {
        Self
    }

    fn handle(&mut self, addr: Self::Input, link: &Link<Self>) {
        let listener = TcpListener::bind(&addr).expect("Failed to bind");
        // Notify we're ready to accept connections.
        link.respond(()).unwrap();
        loop {
            if let Ok(stream) = listener.accept() {
                // Spawn a server for this connection
                wactor::spawn::<Server>().send(stream).unwrap()
            }
        }
    }
}

fn main() {
    let server_url = "127.0.0.1:6000";
    // Start listening
    wactor::spawn::<Listener>()
        // Wait until its ready
        .get(server_url)
        .unwrap();

    // Connect to our server.
    let client = TcpStream::connect(server_url).unwrap();
    let (mut socket, _response) =
        tungstenite::client(format!("ws://{}", server_url), client).expect("Failed to connect");

    // Let the ping pong begin!
    let buf = bincode::serialize(&Message::Ping).unwrap();
    println!("Sending: Ping");
    socket.write_message(buf.into()).unwrap();
    while let Ok(msg) = socket.read_message() {
        if msg.is_binary() {
            let answer = bincode::deserialize::<Message>(&msg.into_data()).unwrap();

            println!("Received: {:?}", answer);

            let buf = bincode::serialize(&answer).unwrap();
            socket.write_message(buf.into()).ok();
        }
    }
    println!("Done");
}
