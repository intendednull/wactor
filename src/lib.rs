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
            link: Link { sender, receiver },
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
    /// Send input message. This fails if the actor has panicked.
    pub fn send(&self, msg: A::Input) -> Result<(), ()> {
        self.sender.send(msg)
    }
    /// Block until a response is received. This fails if the actor has panicked.
    pub fn receive(&self) -> Result<A::Output, ()> {
        self.receiver.receive()
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
}

impl<A: Actor> Link<A> {
    /// Respond with given output message. Fails if recipient has been dropped.
    pub fn respond(&self, msg: A::Output) -> Result<(), ()> {
        self.sender.send(msg)
    }

    fn receive(&self) -> Result<A::Input, ()> {
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
