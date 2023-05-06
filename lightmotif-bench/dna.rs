#![feature(test)]

#[cfg(feature = "bio")]
extern crate bio;
extern crate lightmotif;
extern crate test;

#[cfg(target_feature = "sse2")]
use std::arch::x86_64::__m128;
#[cfg(target_feature = "avx2")]
use std::arch::x86_64::__m256;
use std::str::FromStr;

use lightmotif::Alphabet;
use lightmotif::Background;
use lightmotif::CountMatrix;
use lightmotif::Dna;
use lightmotif::EncodedSequence;
use lightmotif::Pipeline;
use lightmotif::Score;
use lightmotif::StripedScores;

const SEQUENCE: &'static str = include_str!("../lightmotif/benches/ecoli.txt");

#[bench]
fn bench_generic(bencher: &mut test::Bencher) {
    let seq = &SEQUENCE[..10000];
    let encoded = EncodedSequence::<Dna>::from_str(seq).unwrap();
    let mut striped = encoded.to_striped::<32>();

    let bg = Background::<Dna, { Dna::K }>::uniform();
    let cm = CountMatrix::<Dna, { Dna::K }>::from_sequences(&[
        EncodedSequence::from_str("GTTGACCTTATCAAC").unwrap(),
        EncodedSequence::from_str("GTTGATCCAGTCAAC").unwrap(),
    ])
    .unwrap();
    let pbm = cm.to_freq(0.1);
    let pssm = pbm.to_scoring(bg);

    striped.configure(&pssm);
    let mut scores = StripedScores::new_for(&striped, &pssm);
    bencher.bytes = seq.len() as u64;
    bencher.iter(|| {
        Pipeline::<_, f32>::score_into(&striped, &pssm, &mut scores);
        test::black_box(scores.argmax());
    });
}

#[cfg(target_feature = "avx2")]
#[bench]
fn bench_sse2(bencher: &mut test::Bencher) {
    let seq = &SEQUENCE[..10000];
    let encoded = EncodedSequence::<Dna>::from_str(seq).unwrap();
    let mut striped = encoded.to_striped();

    let bg = Background::<Dna, { Dna::K }>::uniform();
    let cm = CountMatrix::<Dna, { Dna::K }>::from_sequences(&[
        EncodedSequence::from_str("GTTGACCTTATCAAC").unwrap(),
        EncodedSequence::from_str("GTTGATCCAGTCAAC").unwrap(),
    ])
    .unwrap();
    let pbm = cm.to_freq(0.1);
    let pssm = pbm.to_scoring(bg);

    striped.configure(&pssm);
    let mut scores = StripedScores::new_for(&striped, &pssm);
    bencher.bytes = seq.len() as u64;
    bencher.iter(|| {
        Pipeline::<_, __m128>::score_into(&striped, &pssm, &mut scores);
        test::black_box(scores.argmax());
    });
}

#[cfg(target_feature = "avx2")]
#[bench]
fn bench_avx2(bencher: &mut test::Bencher) {
    let seq = &SEQUENCE[..10000];
    let encoded = EncodedSequence::<Dna>::from_str(seq).unwrap();
    let mut striped = encoded.to_striped();

    let bg = Background::<Dna, { Dna::K }>::uniform();
    let cm = CountMatrix::<Dna, { Dna::K }>::from_sequences(&[
        EncodedSequence::from_str("GTTGACCTTATCAAC").unwrap(),
        EncodedSequence::from_str("GTTGATCCAGTCAAC").unwrap(),
    ])
    .unwrap();
    let pbm = cm.to_freq(0.1);
    let pssm = pbm.to_scoring(bg);

    striped.configure(&pssm);
    let mut scores = StripedScores::new_for(&striped, &pssm);
    bencher.bytes = seq.len() as u64;
    bencher.iter(|| {
        Pipeline::<_, __m256>::score_into(&striped, &pssm, &mut scores);
        test::black_box(scores.argmax());
    });
}

#[bench]
fn bench_bio(bencher: &mut test::Bencher) {
    use bio::pattern_matching::pssm::DNAMotif;
    use bio::pattern_matching::pssm::Motif;

    let seq = &SEQUENCE[..10000];

    let pssm = DNAMotif::from_seqs(
        vec![b"GTTGACCTTATCAAC".to_vec(), b"GTTGATCCAGTCAAC".to_vec()].as_ref(),
        None,
    )
    .unwrap();

    bencher.bytes = seq.len() as u64;
    bencher.iter(|| test::black_box(pssm.score(seq.as_bytes()).unwrap()));
}
