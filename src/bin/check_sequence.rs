extern crate flate2;
extern crate rayon;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use rayon::prelude::*;
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

struct SeqRange {
    begin: u64,
    end: u64,
}

struct SeqChecker {
    products: BTreeMap<String, SeqRange>,
}

impl SeqChecker {
    fn new() -> SeqChecker {
        SeqChecker{ products: BTreeMap::new() }
    }

    fn update<'b>(&mut self, m: &'b Message) -> Event<'b> {
        match self.products.get_mut(&m.product_id) {
            Some(entry) => {
                let last_sequence = entry.end;
                entry.end = m.sequence;
                if last_sequence + 1 != m.sequence {
                    return Event::Skipped(&m.product_id, last_sequence, m.sequence);
                }
                return Event::Ok;
            },
            None => {},
        }
        self.products.insert(m.product_id.clone(), SeqRange{ begin: m.sequence, end: m.sequence });
        Event::NewProduct(&m.product_id)
    }

    fn into_ranges(self) -> BTreeMap<String, SeqRange> {
        self.products
    }
}

#[derive(Debug)]
enum Event<'a> {
    Ok,
    NewProduct(&'a str),
    Skipped(&'a str, u64, u64),
}

fn process_file(filename: &String) -> BTreeMap<String, SeqRange> {
    let f = File::open(filename).expect("file not found");
    let d = GzDecoder::new(f);
    let b = BufReader::new(d);

    let mut checker = SeqChecker::new();
    let mut products = Vec::<String>::new();
    for line in b.lines() {
        let m: Message = serde_json::from_str(&line.unwrap()).expect("failed to parse JSON");
        match checker.update(&m) {
            Event::Ok => {},
            Event::NewProduct(p) => {
                products.push(p.to_owned());
            },
            Event::Skipped(p, last, new) => {
                println!("{}: Skipped sequence numbers between {} and {} on product {}",
                         filename, last, new, p);
            },
        }
    }

    println!("Finished checking {}. Products: {:?}", filename, products);
    checker.into_ranges()
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let files = &mut args[1..];
    files.sort();

    let ranges: Vec<BTreeMap<String, SeqRange>> = files.par_iter().map(process_file).collect();

    // TODO: I'm sure we could do this more intelligently
    for i in 1..ranges.len() {
        for product in ranges[i-1].keys() {
            if ranges[i-1][product].end + 1 != ranges[i][product].begin {
                println!("Gap detected between files {} and {} on {}: end seq {}, begin seq {}",
                         files[i-1], files[i], product, ranges[i-1][product].end, ranges[i][product].begin);
            }
        }
    }
}
