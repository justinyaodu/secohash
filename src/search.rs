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
use selector::Selector;
use selector_searcher::selector_search;
use selector_searcher::SelectorSearchSolution;

use crate::ir::ExprBuilder;
use crate::ir::Tables;
use crate::ir::Tac;
use crate::ir::Trace;
use crate::spec::Spec;
use crate::util::table_index_mask;
use crate::util::table_size;
use crate::util::to_u32;
use crate::util::to_usize;

pub fn search(spec: &Spec) -> Option<Phf> {
    let start = Instant::now();
    /*
    let SelectorSearchSolution {
        mut tac,
        mut tables,
        sel_regs,
    } = selector_search(spec)?;
    */
    let sels = Selector::search(spec)?;
    eprintln!("found selectors: {sels:?}");
    let mut tac = Tac::new();
    let mut tables = Tables::new();
    let sel_regs: Vec<_> = sels
        .into_iter()
        .map(|sel| sel.compile(&mut tac, &mut tables))
        .collect();
    eprintln!("selector search took {} us", start.elapsed().as_micros());

    let start = Instant::now();
    let trace = Trace::new(&spec.interpreted_keys, &tac, &tables, None);
    eprintln!("trace took {} us", start.elapsed().as_micros());

    let sel_cols: Vec<&[u32]> = sel_regs.iter().map(|&reg| &trace[reg]).collect();
    let start = Instant::now();
    let mixer = Mixer::search(&sel_cols)?;
    eprintln!("mixer search took {} us", start.elapsed().as_micros());
    eprintln!("mixer has {} bits", mixer.mix_bits);

    let mix_reg = mixer.compile(&mut tac, &sel_regs);
    let unmasked_hash_reg = if mixer.mix_bits <= spec.min_hash_bits {
        let size = table_size(spec.min_hash_bits);
        let mask = table_index_mask(spec.min_hash_bits);

        let mut seen = vec![false; size];
        for &mix in &mixer.mixes {
            seen[to_usize(mix & mask)] = true;
        }
        let mut rotation = None;
        for i in 0..size {
            if !seen[0usize.wrapping_sub(i) & to_usize(mask)] {
                rotation = Some(to_u32(i));
                break;
            }
        }
        let rotation = rotation.unwrap();
        let x = ExprBuilder();
        tac.push_expr(x.add(x.reg(mix_reg), x.imm(rotation)))
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
