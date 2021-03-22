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
                self.count += 1;
                link.respond(Output::Count(self.count)).ok();
            }
        }
    }
}

fn main() {
    let one = wactor::spawn::<Counter>();
    let two = one.clone();

    one.send(Input::AddOne).ok();
    two.send(Input::AddOne).ok();

    println!("{:?}", one.receive());
    println!("{:?}", two.receive());
}
