#[derive(Debug, Clone)]
pub struct Conf {
    pub instance_name: String,
    pub debug_channel: String,
    pub api_url: String,
    pub ws_url: String,
    pub token: String,
}

impl Conf {
    pub fn new() -> Result<Self, std::env::VarError> {
        Ok(Self {
            instance_name: std::env::var("BOT_NAME")?,
            debug_channel: std::env::var("BOT_DEBUG_CHAN")?,
            api_url: std::env::var("BOT_API_URL")?,
            ws_url: std::env::var("BOT_WS_URL")?,
            token: std::env::var("BOT_TOKEN")?,
        })
    }
}
