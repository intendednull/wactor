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

    fn handle(&mut self, stream: Self::Input, _link: &Link<Self>) {
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
                }
                Err(_) => break,
                _ => {}
            }
        }
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
    // Start listening
    wactor::spawn::<Listener>()
        // Wait until its ready
        .get("127.0.0.1:6000".to_string())
        .unwrap();

    // Connect to our server.
    let client = TcpStream::connect("127.0.0.1:6000").unwrap();
    let (mut socket, _response) =
        tungstenite::client("ws://127.0.0.1:6000", client).expect("Failed to connect");

    // Let the ping pong begin!
    let buf = bincode::serialize(&Message::Ping).unwrap();
    socket.write_message(buf.into()).unwrap();
    loop {
        let msg = socket.read_message().unwrap();
        if msg.is_binary() {
            let answer = bincode::deserialize::<Message>(&msg.into_data()).unwrap();

            println!("{:?}", answer);

            let buf = bincode::serialize(&answer).unwrap();
            socket.write_message(buf.into()).unwrap();
        }
    }
}
