#[derive(Debug, Clone)]
pub struct Conf {
    pub debug_channel: String,
    pub api_url: String,
    pub ws_url: String,
    pub token: String,
    pub threads: u64,
    pub db_url: String,
}

impl Conf {
    pub fn new() -> Result<Self, std::env::VarError> {
        Ok(Self {
            debug_channel: std::env::var("BOT_DEBUG_CHAN").expect("BOT_DEBUG_CHAN"),
            api_url: std::env::var("BOT_API_URL").expect("BOT_API_URL"),
            ws_url: std::env::var("BOT_WS_URL").expect("BOT_WS_URL"),
            token: std::env::var("BOT_TOKEN").expect("BOT_TOKEN"),
            db_url: std::env::var("BOT_DB_URL").expect("BOT_DB_URL"),
            /*threads: std::env::var("BOT_THREADS")
            .unwrap_or(String::from("1"))
            .parse()
            .unwrap_or(4),*/
            threads: 1,
        })
    }
}