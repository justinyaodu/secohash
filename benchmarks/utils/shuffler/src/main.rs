use std::io;
use std::io::BufRead;

use rand::distributions::{Distribution, Uniform};

fn main() {
    let n = std::env::args().nth(1).unwrap().parse::<usize>().unwrap();

    let mut stdin = io::stdin().lock();
    let mut keys = Vec::new();
    loop {
        let mut line = String::new();
        if stdin.read_line(&mut line).unwrap() == 0 {
            break;
        }
        keys.push(line.trim().to_string());
    }

    let mut rng = rand::thread_rng();
    let dist = Uniform::from(0..keys.len());
    for _ in 0..n {
        println!("{}", keys[dist.sample(&mut rng)]);
    }
}
