//! # Wactor
//! WASM actor system based on [lunatic](https://github.com/lunatic-solutions/lunatic).
//!
//! Actors run on isolated green threads. They cannot share memory, and communicate only through
//! input and output messages. Consequently messages must be serialized to travel between threads.
//!
//! ## Example
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use wactor::*;
//!
//! struct Counter {
//!     count: u32,
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! enum Input {
//!     AddOne,
//! }
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! enum Output {
//!     Count(u32),
//! }
//!
//! impl Actor for Counter {
//!     type Input = Input;
//!     type Output = Output;
//!
//!     fn create() -> Self {
//!         Self { count: 0 }
//!     }
//!
//!     fn handle(&mut self, msg: Self::Input, link: &Link<Self>) {
//!         match msg {
//!             Input::AddOne => {
//!                 // Increment count by 1.
//!                 self.count += 1;
//!                 // Respond with new count. This fails if our recipient has been dropped.
//!                 link.respond(Output::Count(self.count)).ok();
//!             }
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // Spawn our actor. We get a bridge for sending and receiving messages. Can be cloned for
//!     // multiple owners. Actor is dropped after all bridges have been dropped.
//!     let bridge = wactor::spawn::<Counter>();
//!     // Send our input message. This fails if our actor has panicked (unrecoverable error).
//!     bridge.send(Input::AddOne).expect("Dead actor");
//!     // Block until a response is received. This also fails if our actor has panicked.
//!     let result = bridge.receive();
//!     // Assert we received the correct value.
//!     assert_eq!(result, Ok(Output::Count(1)));
//! }
//! ```
//!
//! ### How to run
//! Install lunatic then build and run:
//!
//!     cargo build --release --target=wasm32-wasi --example basic
//!     lunatic target/wasm32-wasi/release/examples/basic.wasm
use std::cell::Cell;

use lunatic::{
    process::{self, Process},
    LunaticError, Mailbox, ReceiveError, Request,
};
use serde::{de::DeserializeOwned, Serialize};

/// Actors run on isolated green threads. The cannot share memory, and communicate only through
/// input and output messages. Consequently messages must be serialized to travel between threads.
pub trait Actor: Sized {
    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;
    type Context: Serialize + DeserializeOwned;

    /// Create this actor.
    fn create(ctx: Self::Context) -> Self;
    /// Handle an input message.
    fn handle(&mut self, msg: &Self::Input, link: &Link<Self>) -> Self::Output;
}

/// Spawn a new [Actor], returning its [Bridge]. Actor is dropped when all bridges have been
/// dropped.
pub fn spawn_with<A: Actor>(
    ctx: A::Context,
) -> Result<Process<Request<<A as Actor>::Input, <A as Actor>::Output>>, LunaticError> {
    process::spawn_with(
        ctx,
        |ctx, mailbox: Mailbox<Request<A::Input, A::Output>>| {
            Context {
                link: Link {
                    mailbox,
                    open: Cell::new(true),
                },
                actor: A::create(ctx),
            }
            .run()
        },
    )
}

pub fn spawn<A: Actor<Context = ()>>(
) -> Result<Process<Request<<A as Actor>::Input, <A as Actor>::Output>>, LunaticError> {
    spawn_with::<A>(())
}

enum LinkError {
    Receive(ReceiveError),
    Closed,
}

/// Link for responding to input messages.
pub struct Link<A: Actor> {
    mailbox: Mailbox<Request<A::Input, A::Output>>,
    // Whether this link may receive messages. Setting this to true will drop actor after it's done
    // handling current message.
    open: Cell<bool>,
}

impl<A: Actor> Link<A> {
    /// Signal this actor should be dropped after handling current message.
    pub fn close(&self) {
        self.open.set(false);
    }

    fn receive(&self) -> Result<Request<A::Input, A::Output>, LinkError> {
        if !self.open.get() {
            return Err(LinkError::Closed);
        }

        self.mailbox.receive().map_err(LinkError::Receive)
    }
}

/// Context for actor execution.
struct Context<A: Actor> {
    link: Link<A>,
    actor: A,
}

impl<A: Actor> Context<A> {
    fn run(mut self) {
        // Receive messages until we get an error, meaning all recipients have been dropped.
        while let Ok(request) = self.link.receive() {
            let response = self.actor.handle(request.data(), &self.link);
            request.reply(response);
        }
    }
}
