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
    channel::{self, Receiver, Sender},
    Process,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Actors run on isolated green threads. The cannot share memory, and communicate only through
/// input and output messages. Consequently messages must be serialized to travel between threads.
pub trait Actor: Sized {
    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;

    /// Create this actor.
    fn create() -> Self;
    /// Handle an input message.
    fn handle(&mut self, msg: Self::Input, link: &Link<Self>);
}

/// Spawn a new [Actor], returning its [Bridge]. Actor is dropped when all bridges have been
/// dropped.
pub fn spawn<A: Actor>() -> Bridge<A> {
    let (in_sender, in_receiver) = channel::unbounded::<A::Input>();
    let (out_sender, out_receiver) = channel::unbounded::<A::Output>();

    Process::spawn_with((in_receiver, out_sender), |(receiver, sender)| {
        Context {
            link: Link {
                sender,
                receiver,
                open: Cell::new(true),
            },
            actor: A::create(),
        }
        .run()
    })
    .detach();

    Bridge {
        sender: in_sender,
        receiver: out_receiver,
    }
}

/// Bridge to an actor. Can be cloned for multiple owners. Actor is dropped when all bridges have
/// been dropped.
#[derive(Serialize, Deserialize)]
pub struct Bridge<A: Actor> {
    sender: Sender<A::Input>,
    receiver: Receiver<A::Output>,
}

impl<A: Actor> Bridge<A> {
    /// Send input message. Fails if the actor has been dropped.
    pub fn send<M>(&self, msg: M) -> Result<(), ()>
    where
        M: Into<A::Input>,
    {
        self.sender.send(msg.into())
    }

    /// Wait until a response is received. Fails if the actor has been dropped.
    ///
    /// # Warning
    ///
    /// The message received is not guaranteed to be a related to last sent. Responses are sent in
    /// the order they're received, so if multiple bridges send messages they may receive something
    /// unexpected. Hypothetically if all bridges only use [Bridge::get]`, they should receive the
    /// related message, however this is untested and unreliable.
    pub fn receive(&self) -> Result<A::Output, ()> {
        self.receiver.receive()
    }

    /// Send a message and wait until for the response is received. Fails if the actor has been
    /// dropped.
    ///
    /// # Warning
    ///
    /// The message received is not guaranteed to be a related to last sent. Responses are sent in
    /// the order they're received, so if multiple bridges send messages they may receive something
    /// unexpected. Hypothetically if all bridges only use [Bridge::get]`, they should receive the
    /// related message, however this is untested and unreliable.
    pub fn get<M>(&self, msg: M) -> Result<A::Output, ()>
    where
        M: Into<A::Input>,
    {
        self.send(msg)?;
        self.receive()
    }
}

impl<A: Actor> Clone for Bridge<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

/// Link for responding to input messages.
pub struct Link<A: Actor> {
    sender: Sender<A::Output>,
    receiver: Receiver<A::Input>,
    // Whether this link may receive messages. Setting this to true will drop actor after it's done
    // handling current message.
    open: Cell<bool>,
}

impl<A: Actor> Link<A> {
    /// Respond with given output message. Fails if recipient has been dropped.
    pub fn respond<M>(&self, msg: M) -> Result<(), ()>
    where
        M: Into<A::Output>,
    {
        self.sender.send(msg.into())
    }

    /// Signal this actor should be dropped after handling current message.
    pub fn close(&self) {
        self.open.set(false);
    }

    fn receive(&self) -> Result<A::Input, ()> {
        if !self.open.get() {
            return Err(());
        }

        self.receiver.receive()
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
        while let Ok(msg) = self.link.receive() {
            self.actor.handle(msg, &self.link);
        }
    }
}
