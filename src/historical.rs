pub mod feed {
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

    /// Finds the chunk of websocket messages starting before start_time.
    /// If found, returns a Stream of messages.
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

    /// Returns a book snapshot from around start_time.
    pub fn snapshot_starting_approx(
        start_time: DateTime<Utc>,
    ) -> Box<Future<Item = HashMap<String, PathBuf>, Error = io::Error>> {
        // TODO: Open snapshots...
        get_best_snapshot_per_product::<DefaultGlob>(start_time)
    }

    fn get_best_snapshot_per_product<G: Glob>(
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
        use std::vec;

        use chrono::{TimeZone, Utc};
        use glob;
        use futures::future::FutureResult;
        use tokio::executor::current_thread;

        #[test]
        fn get_best_snapshot() {
            struct TestGlob;
            impl<'a> Glob for TestGlob {
                type Paths = vec::IntoIter<Result<PathBuf, glob::GlobError>>;
                fn glob(pattern: &str) -> Result<Self::Paths, glob::PatternError> {
                    let files = vec![
                        "data/BTC-USD_20180225_170132.json.gz",
                        "data/BTC-USD_20180225_170017.json.gz",
                        "data/BTC-USD_20170225_170000.json.gz",
                        "data/BTC-EUR_20180225_170000.json.gz",
                    ];
                    let pattern = glob::Pattern::new(pattern)?;
                    let glob_result: Vec<Result<PathBuf, glob::GlobError>> = files
                        .into_iter()
                        .filter(|s| pattern.matches(s))
                        .map(|s| Result::Ok(PathBuf::from(s)))
                        .collect();
                    Result::Ok(glob_result.into_iter())
                }
            }

            current_thread::run(|_| {
                let query = Utc.ymd(2018, 2, 25).and_hms(17, 0, 0);
                let fut = get_best_snapshot_per_product::<TestGlob>(query)
                    .and_then(|result| {
                        let mut expected = HashMap::new();
                        expected.insert(
                            "BTC-USD".into(),
                            PathBuf::from("data/BTC-USD_20180225_170017.json.gz"),
                        );
                        expected.insert(
                            "BTC-EUR".into(),
                            PathBuf::from("data/BTC-EUR_20180225_170000.json.gz"),
                        );
                        assert_eq!(expected, result);
                        Ok(())
                    })
                    .or_else(|_| -> FutureResult<(), ()> {
                        panic!("get_best_snapshot_per_product failed");
                    });
                current_thread::spawn(fut);
            });

            current_thread::run(|_| {
                let query = Utc.ymd(2017, 2, 25).and_hms(17, 0, 0);
                let fut = get_best_snapshot_per_product::<TestGlob>(query)
                    .and_then(|result| {
                        let mut expected = HashMap::new();
                        expected.insert(
                            "BTC-USD".into(),
                            PathBuf::from("data/BTC-USD_20170225_170000.json.gz"),
                        );
                        assert_eq!(expected, result);
                        Ok(())
                    })
                    .or_else(|_| -> FutureResult<(), ()> {
                        panic!("get_best_snapshot_per_product failed");
                    });
                current_thread::spawn(fut);
            });

            current_thread::run(|_| {
                let query = Utc.ymd(2017, 2, 25).and_hms(18, 0, 0);
                let fut = get_best_snapshot_per_product::<TestGlob>(query)
                    .and_then(|result| {
                        let mut expected = HashMap::new();
                        assert_eq!(expected, result);
                        Ok(())
                    })
                    .or_else(|_| -> FutureResult<(), ()> {
                        panic!("get_best_snapshot_per_product failed");
                    });
                current_thread::spawn(fut);
            });
        }
    }
}
