mod hasher;

use std::io;
use std::io::BufRead;

use hasher::Hasher;

fn main() {
    let hasher = Hasher::new();
    let mut stdin = io::stdin().lock();
    let mut line = String::new();
    let mut total: u64 = 0;
    while stdin.read_line(&mut line).unwrap() > 0 {
        line.pop();
        total += hasher.lookup(&line);
        line.clear();
    }
    println!("{total}");
}
