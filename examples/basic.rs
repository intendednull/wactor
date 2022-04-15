use lunatic::Mailbox;
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
    type Context = ();

    fn create(_ctx: Self::Context) -> Self {
        Self { count: 0 }
    }

    fn handle(&mut self, msg: &Self::Input, _link: &Link<Self>) -> Self::Output {
        match msg {
            Input::AddOne => {
                // Increment count by 1.
                self.count += 1;
                // Respond with new count. This fails if our recipient has been dropped.
                Output::Count(self.count)
            }
        }
    }
}

#[lunatic::main]
fn main(_m: Mailbox<()>) {
    // Spawn our actor. We get a bridge for sending and receiving messages. Can be cloned for
    // multiple owners. Actor is dropped after all bridges have been dropped.
    let actor = wactor::spawn::<Counter>().unwrap();
    // Send our input message. This fails if our actor has panicked (unrecoverable error).
    let result = actor.request(Input::AddOne).expect("Dead actor");
    // Assert we received the correct value.
    assert_eq!(result, Output::Count(1));
}
