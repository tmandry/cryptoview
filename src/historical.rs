mod feed {
    use std::collections::{hash_map, HashMap};
    use std::io;
    use std::io::{BufRead, BufReader};
    use std::fs::File;
    use std::path::PathBuf;

    use chrono::{DateTime, Utc};
    use flate2::read::GzDecoder;
    use futures::{future, stream, Future, Stream};
    use glob::glob;

    pub fn chunk_starting_approx<'a>(
        start_time: DateTime<Utc>,
    ) -> Result<Box<Stream<Item = String, Error = io::Error>>, io::Error> {
        let filename = start_time
            .format("data/ws_%Y%m%d_%H0000.txt.gz")
            .to_string();
        let r = BufReader::new(GzDecoder::new(File::open(filename)?));
        let stream = stream::iter_result(r.lines());
        Ok(Box::new(stream))
    }

    pub fn snapshot_starting_approx(
        start_time: DateTime<Utc>,
    ) -> Box<Future<Item = HashMap<String, PathBuf>, Error = io::Error>> {
        let pattern = start_time
            .format("data/???-???_%Y%m%d_%H????.json.gz")
            .to_string();

        // Find the lexicographically smallest filename (earliest time) for each product.
        let mut products = HashMap::new();
        for entry in glob(&pattern).expect("Bad glob!") {
            match entry {
                Ok(path) => {
                    let product = path.file_name().unwrap().to_str().unwrap()[..7].to_owned();
                    match products.entry(product) {
                        hash_map::Entry::Vacant(v) => {
                            v.insert(path);
                        }
                        hash_map::Entry::Occupied(mut o) => {
                            if path.file_name() < o.get().file_name() {
                                *o.get_mut() = path;
                            }
                        }
                    }
                }
                Err(e) => println!("Error while globbing: {:?}", e),
            }
        }
        Box::new(future::ok(products))
    }
}

#[cfg(test)]
mod test {
    use super::feed::*;
    use chrono::{DateTime, TimeZone, Utc};
    use futures::Future;

    #[test]
    fn test() {
        let result = snapshot_starting_approx(Utc.ymd(2017, 11, 9).and_hms(0, 0, 0));
        panic!("{:?}", result.wait().unwrap());
    }
}
