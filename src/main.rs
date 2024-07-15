mod backend;
mod combinatorics;
mod frontend;
mod optimizer;
mod phf;
mod search;

use std::io;
use std::io::BufRead;

use backend::{Backend, CBackend};
use phf::Phf;
use search::search;

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

    let phf = Phf::new(&keys);
    let phf = search(&phf).expect("search failed");

    let c_code = CBackend::new().emit(&phf);
    println!("{}", c_code);
}
