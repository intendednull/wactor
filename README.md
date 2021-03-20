# Wactor
WASM actor system based on lunatic

```rust
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

    fn update(&mut self, msg: Self::Input, responder: &Responder<Self>) {
        match msg {
            Input::AddOne => {
                self.count += 1;
                responder.respond(Output::Count(self.count));
            }
        }
    }
}

fn main() {
    let link = wactor::run::<Counter>();
    link.send(Input::AddOne);
    let result = link.receive();
    assert_eq!(result, Output::Count(1));
}
```
