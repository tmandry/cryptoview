extern crate flate2;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use std::env;
use std::collections::BTreeMap;
//use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::{BufRead, BufReader};
use flate2::read::GzDecoder;

#[derive(Deserialize, Debug)]
struct Message {
    sequence: u64,
    product_id: String,
}

struct SequenceChecker {
    products: BTreeMap<String, u64>,
}

impl SequenceChecker {
    fn new() -> SequenceChecker {
        SequenceChecker{ products: BTreeMap::new() }
    }

    fn update<'b>(&mut self, m: &'b Message) -> Event<'b> {
        match self.products.get_mut(&m.product_id) {
            Some(id) => {
                let last_sequence = *id;
                *id = m.sequence;
                if last_sequence + 1 != m.sequence {
                    return Event::Skipped(&m.product_id, last_sequence, *id);
                }
                return Event::Ok;
            },
            None => {},
        }
        self.products.insert(m.product_id.clone(), m.sequence);
        Event::NewProduct(&m.product_id)
    }
}

#[derive(Debug)]
enum Event<'a> {
    Ok,
    NewProduct(&'a str),
    Skipped(&'a str, u64, u64),
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let f = File::open(filename).expect("file not found");
    let d = GzDecoder::new(f);
    let b = BufReader::new(d);

    let mut checker = SequenceChecker::new();
    for line in b.lines() {
        let m: Message = serde_json::from_str(&line.unwrap()).expect("failed to parse JSON");
        match checker.update(&m) {
            Event::Ok => {},
            Event::NewProduct(p) => println!("New product {}", p),
            Event::Skipped(p, last, new) => println!("Skipped sequence numbers between {} and {} on product {}", last, new, p),
        }
    }
}
