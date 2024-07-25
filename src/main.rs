mod backend;
mod combinatorics;
mod frontend;
mod ir;
mod search;
mod spec;
mod util;

use std::io;
use std::io::BufRead;

use backend::{Backend, CBackend};
use search::search;
use spec::Spec;

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

    let spec = Spec::new(keys);
    let phf = search(&spec).expect("search failed");

    let c_code = CBackend::new().emit(&spec, &phf);
    println!("{}", c_code);
}
