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
        let mut ws = server::accept(stream).expect("Error creating WS");
        link.respond(()).ok();
        loop {
            let msg = ws.read_message();
            match msg {
                Ok(tungstenite::Message::Binary(msg)) => {
                    let msg =
                        bincode::deserialize::<Message>(&msg).expect("Error deserializing message");

                    let response = match msg {
                        Message::Ping => Message::Pong,
                        Message::Pong => Message::Ping,
                    };

                    let buf = bincode::serialize(&response).expect("Error serializing response");
                    ws.write_message(buf.into())
                        .expect("Error sending response");
                }
                Err(_) => break,
                _ => {}
            }
        }
    }
}

struct Listener {
    listener: TcpListener,
}

impl Actor for Listener {
    type Input = ();
    type Output = ();

    fn create() -> Self {
        Self {
            listener: TcpListener::bind("127.0.0.1:6000").expect("Could not bind"),
        }
    }

    fn handle(&mut self, _msg: Self::Input, link: &Link<Self>) {
        link.respond(()).unwrap();
        loop {
            if let Ok(stream) = self.listener.accept() {
                wactor::spawn::<Server>()
                    .get(stream)
                    .expect("Error spawning node")
            }
        }
    }
}

fn main() {
    // Start listening
    wactor::spawn::<Listener>()
        // Wait until its ready
        .get(())
        .expect("Listener failed to start");

    // Connect to our server.
    let client = TcpStream::connect("127.0.0.1:6000").unwrap();
    let (mut socket, _response) =
        tungstenite::client("ws://127.0.0.1:6000", client).expect("Can't connect");

    // Let the ping pong begin!
    let buf = bincode::serialize(&Message::Ping).unwrap();
    socket.write_message(buf.into()).unwrap();
    loop {
        let msg = socket.read_message().expect("Error reading message");
        if msg.is_binary() {
            let answer = bincode::deserialize::<Message>(&msg.into_data()).unwrap();
            println!("{:?}", answer);

            let buf = bincode::serialize(&answer).expect("Error serializing response");
            socket.write_message(buf.into()).unwrap();
        }
    }
}
