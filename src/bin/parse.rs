extern crate flate2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::env;
//use std::io::prelude::*;
use std::fs::File;
use flate2::read::GzDecoder;

#[derive(Deserialize, Debug)]
struct BookSnapshot {
    sequence: u64,
    #[allow(dead_code)] bids: Vec<Vec<String>>,
    #[allow(dead_code)] asks: Vec<Vec<String>>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let f = File::open(filename).expect("file not found");
    let d = GzDecoder::new(f);
    let v: BookSnapshot = serde_json::from_reader(d).expect("failed to parse JSON");
    //let mut s = String::new();
    //d.read_to_string(&mut s).unwrap();
    println!("{:?}", v);
}
