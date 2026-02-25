pub struct Config {
    // pub is_production: bool,
    pub backend_url: &'static str,
}

pub const ENV: Config = Config {
    // is_production: false,
    backend_url: "http://127.0.0.1:3000",
};
