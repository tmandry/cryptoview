mod feed {
    use std::borrow::BorrowMut;
    use std::collections::{hash_map, HashMap};
    use std::io;
    use std::io::{BufRead, BufReader, Read};
    use std::fs::File;
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, Utc};
    use flate2::read::GzDecoder;
    use futures::{future, stream, Future, Stream};
    use glob;
    //use glob::glob;

    trait Glob {
        //type IterItem: BorrowMut<glob::GlobResult>;
        type Paths: Iterator<Item = glob::GlobResult>;
        fn glob(pattern: &str) -> Result<Self::Paths, glob::PatternError>;
    }

    struct DefaultGlob;
    impl Glob for DefaultGlob {
        //type IterItem = glob::GlobResult;
        type Paths = glob::Paths;
        fn glob(pattern: &str) -> Result<glob::Paths, glob::PatternError> {
            glob::glob(&pattern)
        }
    }

    trait Open {
        type F: Read;
        fn open<P: AsRef<Path>>(path: P) -> io::Result<Self::F>;
    }

    struct DefaultOpen;
    impl Open for DefaultOpen {
        type F = File;
        fn open<P: AsRef<Path>>(path: P) -> io::Result<Self::F> {
            File::open(path)
        }
    }

    pub fn chunk_starting_approx<'a>(
        start_time: DateTime<Utc>,
    ) -> Result<Box<Stream<Item = String, Error = io::Error>>, io::Error> {
        chunk_starting_approx_impl::<DefaultOpen>(start_time)
    }

    fn chunk_starting_approx_impl<'a, F: Open + 'static>(
        start_time: DateTime<Utc>,
    ) -> Result<Box<Stream<Item = String, Error = io::Error>>, io::Error> {
        let filename = start_time
            .format("data/ws_%Y%m%d_%H0000.txt.gz")
            .to_string();
        let r = BufReader::new(GzDecoder::new(F::open(filename)?));
        let stream = stream::iter_result(r.lines());
        Ok(Box::new(stream))
    }

    pub fn snapshot_starting_approx(
        start_time: DateTime<Utc>,
    ) -> Box<Future<Item = HashMap<String, PathBuf>, Error = io::Error>> {
        snapshot_starting_approx_impl::<DefaultGlob>(start_time)
    }

    fn snapshot_starting_approx_impl<G: Glob>(
        start_time: DateTime<Utc>,
    ) -> Box<Future<Item = HashMap<String, PathBuf>, Error = io::Error>> {
        let pattern = start_time
            .format("data/???-???_%Y%m%d_%H????.json.gz")
            .to_string();

        // Find the lexicographically smallest filename (earliest time) for each product.
        let mut products = HashMap::new();
        for mut entry in G::glob(&pattern).expect("Bad glob!") {
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

    #[cfg(test)]
    mod test {
        use super::*;

        use std::path::PathBuf;
        use std::slice;
        use std::mem;
        use std::vec;

        use chrono::{TimeZone, Utc};
        use futures::Future;
        use glob;

        #[test]
        fn test() {
            struct TestGlob;
            impl<'a> Glob for TestGlob {
                type Paths = vec::IntoIter<Result<PathBuf, glob::GlobError>>;
                fn glob(pattern: &str) -> Result<Self::Paths, glob::PatternError> {
                    let glob_result: Vec<Result<PathBuf, glob::GlobError>> =
                        vec![Result::Ok(PathBuf::from("data/blahblah.json.gz"))];
                    Result::Ok(glob_result.into_iter())
                }
            }
            let result =
                snapshot_starting_approx_impl::<TestGlob>(Utc.ymd(2017, 11, 9).and_hms(0, 0, 0));
            panic!("{:?}", result.wait().unwrap());
        }
    }
}
