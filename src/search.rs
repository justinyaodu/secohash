mod compressor;
mod compressor_searcher;
mod generational_bit_set;
mod mixer;
mod phf;
mod selector;
mod selector_searcher;

use std::time::Instant;

use compressor::Compressor;
use compressor_searcher::CompressorSearchSolution;
use mixer::Mixer;
pub use phf::Phf;
use selector_searcher::selector_search;
use selector_searcher::SelectorSearchSolution;

use crate::ir::ExprBuilder;
use crate::ir::Trace;
use crate::spec::Spec;

pub fn search(spec: &Spec) -> Option<Phf> {
    let start = Instant::now();
    let SelectorSearchSolution {
        mut tac,
        mut tables,
        sel_regs,
    } = selector_search(spec)?;
    eprintln!("selector search took {} us", start.elapsed().as_micros());

    let start = Instant::now();
    let trace = Trace::new(&spec.interpreted_keys, &tac, &tables, None);
    eprintln!("trace took {} us", start.elapsed().as_micros());

    let sel_cols: Vec<&[u32]> = sel_regs.iter().map(|&reg| &trace[reg]).collect();
    let start = Instant::now();
    let mixer = Mixer::search(&sel_cols)?;
    eprintln!("mixer search took {} us", start.elapsed().as_micros());

    let mix_reg = mixer.compile(&mut tac, &sel_regs);
    let unmasked_hash_reg = if mixer.mix_bits == spec.min_hash_bits && !mixer.uses_index_zero {
        mix_reg
    } else {
        let start = Instant::now();
        let compressor = Compressor::search(spec, &mixer)?;
        eprintln!("compressor search took {} ms", start.elapsed().as_millis());
        compressor.compile(&mut tac, &mut tables, mix_reg)
    };

    let x = ExprBuilder();
    tac.push_expr(x.and(x.reg(unmasked_hash_reg), x.hash_mask()));

    Some(Phf::new(
        spec,
        CompressorSearchSolution {
            tac,
            tables,
            hash_bits: spec.min_hash_bits,
        },
    ))
}
