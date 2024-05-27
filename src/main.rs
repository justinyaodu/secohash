mod backend;
mod compressor_searcher;
mod ir;
mod keys;
mod selector;
mod selector_searcher;

use std::io;
use std::io::BufRead;

use backend::{Backend, CBackend};
use compressor_searcher::compressor_search;
use ir::{Ir, Reg};
use keys::Keys;
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

    let keys = Keys::new(&keys);

    let sels = selector_search(&keys).expect("selector search failed");

    let mut ir = Ir::new();
    let sel_regs: Vec<Reg> = sels.iter().map(|s| s.compile(&mut ir)).collect();

    let (ir, table) =
        compressor_search(&keys, &ir, &sel_regs, keys.num_keys() * 8).expect("compressor search failed");

    let c_code = CBackend::new().emit(&keys, &ir, &table);
    println!("{}", c_code)
}
