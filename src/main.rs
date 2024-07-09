mod backend;
mod comb;
mod compressor_searcher;
mod phf;
mod selector;
mod selector_searcher;
mod shift_gen;

use std::io;
use std::io::BufRead;

use backend::{Backend, CBackend};
use compressor_searcher::compressor_search;
use phf::{Phf, Reg};
use selector_searcher::selector_search;

fn main() {
    let mut stdin = io::stdin().lock();
    let mut keys: Vec<Vec<u32>> = Vec::new();
    loop {
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap() == 0 {
            break;
        }
        keys.push(line.trim().bytes().map(|c| c.into()).collect());
    }

    let mut phf = Phf::new(&keys);

    let sels = selector_search(&phf).expect("selector search failed");
    let sel_regs: Vec<Reg> = sels.iter().map(|s| s.compile(&mut phf)).collect();
    // assert!(ir.distinguishes(&keys, &sel_regs, 32));

    let max_table_size = phf.keys.len() * 4;
    let phf = compressor_search(&phf, &sel_regs, max_table_size).expect("compressor search failed");

    let c_code = CBackend::new().emit(&phf);
    println!("{}", c_code);
}
