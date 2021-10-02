use crate::client;
use crate::handlers::Handler;
use crate::middleware::Continue;
use crate::middleware::Error as MiddlewareError;
use crate::middleware::Middleware as MMiddleware;
use crate::models::{Event, Post, StatusCode, StatusError};
use crossbeam_channel::{Receiver, RecvTimeoutError};
use std::convert::From;

use std::time::Duration;

#[derive(Debug)]
pub enum Error {
    // FIXME: strip down to Fatal and Error
    Other(String),
    Middleware(MiddlewareError),
    Processing(String),
    Client(client::Error),
    Consumer(String),
    Status(String),
}

fn client_err(ce: client::Error) -> Error {
    Error::Client(ce)
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Instance got a fatal error: {:?}", self)
    }
}

impl From<client::Error> for Error {
    fn from(e: client::Error) -> Self {
        Error::Client(e)
    }
}

impl From<MiddlewareError> for Error {
    fn from(e: MiddlewareError) -> Self {
        Error::Middleware(e)
    }
}

pub type PostHandler = Box<dyn Handler<Data = Post> + Send + Sync>;
pub type Middleware = Box<dyn MMiddleware + Send + Sync>;

pub struct MutexedPostHandler<PH> {
    handler: std::sync::Mutex<PH>,
}

impl<PH> MutexedPostHandler<PH> {
    pub fn from(ph: PH) -> Self {
        Self {
            handler: std::sync::Mutex::new(ph),
        }
    }
}

impl<PH: Handler> Handler for MutexedPostHandler<PH> {
    type Data = PH::Data;

    fn name(&self) -> String {
        self.handler.lock().unwrap().name()
    }

    fn help(&self) -> Option<String> {
        self.handler.lock().unwrap().help()
    }

    fn handle(&self, data: &PH::Data) -> crate::handlers::Result {
        self.handler.lock().unwrap().handle(data)
    }
}

pub struct Instance<C> {
    middlewares: Vec<Middleware>,
    post_handlers: Vec<PostHandler>,
    helps: std::collections::HashMap<String, String>,
    client: C,
}

impl<C: client::Sender + client::Notifier> Instance<C> {
    pub fn new(client: C) -> Self {
        Instance {
            middlewares: Vec::new(),
            post_handlers: Vec::new(),
            helps: std::collections::HashMap::new(),
            client,
        }
    }

    pub fn add_middleware(&mut self, middleware: Middleware) -> &mut Self {
        self.middlewares.push(middleware);
        self
    }

    pub fn add_post_handler(&mut self, handler: PostHandler) -> &mut Self {
        handler.help().and_then(|help| {
            self.helps
                .insert(handler.name().to_string(), help.to_string())
        });
        self.post_handlers.push(handler);
        self
    }

    fn process_middlewares(&mut self, event: Event) -> Result<Option<Event>, Error> {
        let mut event = event;
        for middleware in self.middlewares.iter() {
            match middleware.process(event)? {
                Continue::Yes(nevent) => {
                    event = nevent;
                }
                Continue::No => {
                    return Ok(None);
                }
            };
        }

        Ok(Some(event))
    }

    fn process_help(&self, post: &Post) -> Result<(), Error> {
        if &post.message == "!help" {
            let mut reply = String::new();
            let mut keys: Vec<String> = self.helps.keys().map(|v| v.clone()).collect();
            keys.sort();
            for key in keys.iter() {
                reply.push_str(&format!("`{}`\n", key));
            }

            return self.client.reply(post, &reply).map_err(client_err);
        }

        match regex::Regex::new("^!help ([a-zA-Z0-9_-]+).*")
            .unwrap()
            .captures(&post.message)
        {
            Some(captures) => {
                let name = captures.get(1).unwrap().as_str();
                match self.helps.get(name) {
                    Some(m) => self.client.reply(post, m),
                    None => self.client.reply(post, "tutétrompé"),
                }
                .map_err(client_err)
            }
            None => Ok(()),
        }
    }

    fn process_event_post(&mut self, post: Post) -> Result<(), Error> {
        let _ = self.process_help(&post)?;
        for handler in self.post_handlers.iter_mut() {
            let res = handler.handle(&post);
            let _ = match res {
                Ok(_) => {}
                Err(e) => match self.client.debug(&format!("error: {:?}", e)) {
                    Ok(_) => {}
                    Err(e) => println!("debug error: {:?}", e),
                },
            };
        }
        Ok(())
    }

    fn process_event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            Event::Post(post) => self.process_event_post(post),
            Event::PostEdited(_edited) => {
                println!("edits are unsupported for now");
                Ok(())
            }
            Event::Unsupported(_unsupported) => {
                //println!("unsupported event: {:?}", unsupported);
                Ok(())
            }
            Event::Hello(hello) => {
                println!("hello server {:?}", hello.server_string);
                Ok(())
            }
            Event::Status(status) => match status.code {
                StatusCode::OK => Ok(()),
                StatusCode::Error => Err(Error::Status(
                    status.error.unwrap_or(StatusError::new_none()).message,
                )),
                StatusCode::Unsupported => {
                    println!("unsupported: {:?}", status);
                    Ok(())
                }
                StatusCode::Unknown => Err(Error::Other(
                    status.error.unwrap_or(StatusError::new_none()).message,
                )),
            },
            Event::Shutdown => Ok(()), // should not arrive here
        }
    }

    fn process(&mut self, event: Event) -> Result<(), Error> {
        let res = self.process_middlewares(event)?;
        match res {
            Some(event) => self.process_event(event),
            None => Ok(()),
        }
    }

    pub fn run(&mut self, receiver: Receiver<Event>) -> Result<(), Error> {
        let mut loaded = String::from("## Loaded middlewares\n");
        for m in self.middlewares.iter() {
            loaded.push_str(&format!(" * `{}`\n", m.name()));
        }
        loaded.push_str("## Loaded post handlers\n");
        for h in self.post_handlers.iter() {
            loaded.push_str(&format!(" * `{}`\n", h.name()));
        }

        let _ = self.client.startup(&loaded)?;

        loop {
            match receiver.recv_timeout(Duration::from_secs(5)) {
                Ok(e) => match e {
                    Event::Shutdown => return Ok(()),
                    _ => self.process(e)?,
                },
                Err(rte) => match rte {
                    RecvTimeoutError::Timeout => {}
                    RecvTimeoutError::Disconnected => {
                        return Err(Error::Consumer(format!(
                            "receiving channel closed"
                        )));
                    }
                },
            };
        }
    }
}
