use serde::{Deserialize, Serialize};
use wactor::*;

struct Counter {
    count: u32,
}

#[derive(Serialize, Deserialize)]
enum Input {
    AddOne,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Output {
    Count(u32),
}

impl Actor for Counter {
    type Input = Input;
    type Output = Output;

    fn create() -> Self {
        Self { count: 0 }
    }

    fn handle(&mut self, msg: Self::Input, link: &Link<Self>) {
        match msg {
            Input::AddOne => {
                // Increment count by 1.
                self.count += 1;
                // Respond with new count. This fails if our recipient has been dropped.
                link.respond(Output::Count(self.count)).ok();
            }
        }
    }
}

fn main() {
    // Spawn our actor. We get a bridge for sending and receiving messages. Can be cloned for
    // multiple owners. Actor is dropped after all bridges have been dropped.
    let bridge = wactor::spawn::<Counter>();
    // Send our input message. This fails if our actor has panicked (unrecoverable error).
    bridge.send(Input::AddOne).expect("Dead actor");
    // Block until a response is received. This also fails if our actor has panicked.
    let result = bridge.receive();
    // Assert we received the correct value.
    assert_eq!(result, Ok(Output::Count(1)));
}
