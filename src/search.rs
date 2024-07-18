use std::time::Instant;

use compressor_searcher::compressor_search;
use selector_searcher::selector_search;

use crate::phf::Phf;

mod compressor_searcher;
mod selector;
mod selector_searcher;

pub fn search(phf: &Phf) -> Option<Phf> {
    let start = Instant::now();
    let (phf, sel_regs) = selector_search(phf)?;
    eprintln!("selector search took {} ms", start.elapsed().as_millis());

    let start = Instant::now();
    let ret = compressor_search(&phf, &sel_regs);
    eprintln!("compressor search took {} ms", start.elapsed().as_millis());
    ret
}
