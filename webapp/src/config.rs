#[cfg(debug_assertions)]
mod config_dev;
#[cfg(debug_assertions)]
pub use config_dev::ENV;

#[cfg(not(debug_assertions))]
mod config_prod;
#[cfg(not(debug_assertions))]
pub use config_prod::ENV;
