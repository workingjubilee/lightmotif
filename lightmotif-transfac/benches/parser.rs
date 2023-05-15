#![feature(test)]

extern crate lightmotif;
extern crate lightmotif_transfac;
extern crate test;

use lightmotif::Alphabet;
use lightmotif::Background;
use lightmotif::CountMatrix;
use lightmotif::Dna;
use lightmotif::EncodedSequence;
use lightmotif::Pipeline;
use lightmotif_transfac::reader::Reader;

#[bench]
fn bench_reader(bencher: &mut test::Bencher) {
    let prodoric = include_str!("prodoric.transfac");
    bencher.bytes = prodoric.as_bytes().len() as u64;
    bencher.iter(|| {
        test::black_box(Reader::<_, Dna>::new(std::io::Cursor::new(prodoric)).collect::<Vec<_>>());
    })
}
