use lunatic::{
    channel::{self, Receiver, Sender},
    Process,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub fn run<A: Actor>() -> Link<A> {
    let (in_sender, in_receiver) = channel::unbounded::<A::Input>();
    let (out_sender, out_receiver) = channel::unbounded::<A::Output>();

    Process::spawn_with((in_receiver, out_sender), |(receiver, sender)| {
        Context {
            receiver,
            responder: Responder { sender },
            actor: A::create(),
        }
        .run()
    })
    .detach();

    Link {
        sender: in_sender,
        receiver: out_receiver,
    }
}

pub struct Responder<A: Actor> {
    sender: Sender<A::Output>,
}

impl<A: Actor> Responder<A> {
    pub fn respond(&self, msg: A::Output) {
        self.sender.send(msg).ok();
    }
}

struct Context<A: Actor> {
    responder: Responder<A>,
    receiver: Receiver<A::Input>,
    actor: A,
}

impl<A: Actor> Context<A> {
    fn run(mut self) {
        while let Ok(msg) = self.receiver.receive() {
            self.actor.update(msg, &self.responder);
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Link<A: Actor> {
    sender: Sender<A::Input>,
    receiver: Receiver<A::Output>,
}

impl<A: Actor> Link<A> {
    pub fn send(&self, msg: A::Input) {
        self.sender.send(msg).expect("Channel closed");
    }

    pub fn receive(&self) -> A::Output {
        self.receiver.receive().expect("Channel closed")
    }
}

impl<A: Actor> Clone for Link<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

pub trait Actor: Sized {
    type Input: Serialize + DeserializeOwned;
    type Output: Serialize + DeserializeOwned;

    fn create() -> Self;
    fn update(&mut self, msg: Self::Input, responder: &Responder<Self>);
}
