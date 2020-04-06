use crossbeam::crossbeam_channel::unbounded;
use crossbeam::sync::WaitGroup;
use dotenv;
use flobot::client::*;
use flobot::conf::Conf;
use flobot::db;
use flobot::db::sqlite as dbs;
use flobot::db::tempo::Tempo;
use flobot::handlers;
use flobot::instance::Instance;
use flobot::mattermost::Mattermost;
use flobot::middleware;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    dotenv::from_filename("flobot.env").ok();
    let cfg = Conf::new().expect("cfg err");
    let db_url = cfg.db_url.as_str();

    let (sender, receiver) = unbounded();
    let wg = WaitGroup::new();

    {
        let wg = wg.clone();
        let cfg = cfg.clone();
        thread::spawn(move || {
            println!("launch client thread");
            Mattermost::new(cfg).listen(sender);
            drop(wg);
        });
    }

    println!("run db migrations");
    let conn = db::conn(db_url);
    db::run_migrations(&conn)?;

    println!("launch bot!");
    let botdb = Rc::new(dbs::Sqlite::new(conn));
    let tempo = Tempo::new();
    let ignore_self = Box::new(middleware::IgnoreSelf::new());
    let trigger = Box::new(handlers::trigger::Trigger::new(
        Rc::clone(&botdb),
        tempo.clone(),
        Duration::from_secs(120),
    ));
    let edits = Box::new(handlers::edits::Edit::new(Rc::clone(&botdb)));
    Instance::new(Mattermost::new(cfg.clone()))
        //.add_middleware(Box::new(middleware::Debug::new("debug")))
        .add_middleware(ignore_self)
        //.add_post_handler(Box::new(trigger))
        .add_post_handler(trigger)
        .add_post_handler(edits)
        .run(receiver.clone())?;

    drop(botdb);
    println!("waiting for listener to stop");
    wg.wait();

    Ok(())
}
