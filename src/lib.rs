extern crate chrono;
extern crate flate2;
extern crate futures;
extern crate glob;
extern crate serde;
extern crate tokio;

pub mod book;
pub mod historical;

mod price;
pub type Price = price::Price;
