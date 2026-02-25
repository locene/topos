pub struct Config {
    // pub is_production: bool,
    pub backend_url: &'static str,
}

pub const ENV: Config = Config {
    // is_production: true,
    backend_url: "https://topos.locene.com/api",
};
