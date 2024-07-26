mod bit_set;
mod compressor_searcher;
mod phf;
mod selector;
mod selector_searcher;

use std::time::Instant;

use compressor_searcher::compressor_search;
pub use phf::Phf;
use selector_searcher::selector_search;

use crate::spec::Spec;

pub fn search(spec: &Spec) -> Option<Phf> {
    let start = Instant::now();
    let sol = selector_search(spec)?;
    eprintln!("selector search took {} ms", start.elapsed().as_millis());

    let start = Instant::now();
    let sol = compressor_search(spec, sol)?;
    eprintln!("compressor search took {} ms", start.elapsed().as_millis());

    Some(Phf::new(spec, sol))
}
