use compressor_searcher::compressor_search;
use selector_searcher::selector_search;

use crate::phf::Phf;

mod compressor_searcher;
mod selector;
mod selector_searcher;

pub fn search(phf: &Phf) -> Option<Phf> {
    let (phf, sel_regs) = selector_search(phf)?;

    let max_table_size = phf.keys.len() * 4;
    compressor_search(&phf, &sel_regs, max_table_size)
}
