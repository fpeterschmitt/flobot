use crate::client;
use crate::models::Post;
use std::convert::From;

#[derive(Debug)]
pub enum Error {
    Database(String),
    Timeout(String),
    Status(String),
    Other(String),
    Reaction(String),
    Reply(String),
}

impl From<client::Error> for Error {
    fn from(e: client::Error) -> Self {
        match e {
            client::Error::Timeout(e) => Error::Timeout(e.to_string()),
            client::Error::Other(e) => Error::Other(e.to_string()),
            client::Error::Status(e) => Error::Status(e.to_string()),
            client::Error::Body(e) => Error::Other(e.to_string()),
        }
    }
}

pub type Result = std::result::Result<(), Error>;

pub trait Handler {
    type Data;
    fn name(&self) -> String;
    fn help(&self) -> Option<String>;
    fn handle(&self, data: &Self::Data) -> Result;
}

pub struct Debug {
    name: String,
}

impl Debug {
    pub fn new(name: &str) -> Self {
        Debug {
            name: String::from(name),
        }
    }
}

impl Handler for Debug {
    type Data = Post;

    fn name(&self) -> String {
        "debug".into()
    }
    fn help(&self) -> Option<String> {
        None
    }

    fn handle(&self, post: &Post) -> Result {
        println!("handler {:?} -> {:?}", self.name, post);
        Ok(())
    }
}