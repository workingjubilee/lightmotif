#[cfg(target_feature = "avx2")]
use std::arch::x86_64::*;

use self::seal::Vector;
use super::abc::Alphabet;
use super::abc::DnaAlphabet;
use super::abc::Symbol;
use super::dense::DenseMatrix;
use super::pwm::WeightMatrix;
use super::seq::EncodedSequence;
use super::seq::StripedSequence;

mod seal {
    pub trait Vector {}

    impl Vector for f32 {}

    #[cfg(target_feature = "avx2")]
    impl Vector for std::arch::x86_64::__m256 {}
}

pub struct Pipeline<A: Alphabet, V: Vector> {
    alphabet: A,
    vector: std::marker::PhantomData<V>,
}

impl<A: Alphabet, V: Vector> Pipeline<A, V> {
    pub fn new() -> Self {
        Self {
            alphabet: A::default(),
            vector: std::marker::PhantomData,
        }
    }
}

impl Pipeline<DnaAlphabet, f32> {
    pub fn score<const C: usize>(
        &self,
        seq: &StripedSequence<DnaAlphabet, C>,
        pwm: &WeightMatrix<DnaAlphabet, { DnaAlphabet::K }>,
    ) -> StripedScores<f32, C> {
        let seq_rows = seq.data.rows() - seq.wrap;
        let mut result = DenseMatrix::<f32, C>::new(seq_rows);

        for i in 0..seq.length - pwm.len() + 1 {
            let mut score = 0.0;
            for j in 0..pwm.len() {
                let offset = i + j;
                let col = offset / seq_rows;
                let row = offset % seq_rows;
                score += pwm.data[j][seq.data[row][col].as_index()];
            }
            let col = i / result.rows();
            let row = i % result.rows();
            result[row][col] = score;
        }
        StripedScores {
            length: seq.length - pwm.len() + 1,
            data: result,
            marker: std::marker::PhantomData,
        }
    }
}

#[cfg(target_feature = "avx2")]
impl Pipeline<DnaAlphabet, __m256> {
    pub fn score(
        &self,
        seq: &StripedSequence<DnaAlphabet, { std::mem::size_of::<__m256i>() }>,
        pwm: &WeightMatrix<DnaAlphabet, { DnaAlphabet::K }>,
    ) -> StripedScores<__m256, { std::mem::size_of::<__m256i>() }> {
        const S: i32 = std::mem::size_of::<f32>() as i32;
        const C: usize = std::mem::size_of::<__m256i>();
        const K: usize = DnaAlphabet::K;

        if (seq.wrap < pwm.len() - 1) {
            panic!("not enough wrapping rows for motif of length {}", pwm.len());
        }

        let mut result = DenseMatrix::new(seq.data.rows() - seq.wrap);
        unsafe {
            // get raw pointers to data
            // mask vectors for broadcasting:
            let m1: __m256i = _mm256_set_epi32(
                0xFFFFFF03u32 as i32,
                0xFFFFFF02u32 as i32,
                0xFFFFFF01u32 as i32,
                0xFFFFFF00u32 as i32,
                0xFFFFFF03u32 as i32,
                0xFFFFFF02u32 as i32,
                0xFFFFFF01u32 as i32,
                0xFFFFFF00u32 as i32,
            );
            let m2: __m256i = _mm256_set_epi32(
                0xFFFFFF07u32 as i32,
                0xFFFFFF06u32 as i32,
                0xFFFFFF05u32 as i32,
                0xFFFFFF04u32 as i32,
                0xFFFFFF07u32 as i32,
                0xFFFFFF06u32 as i32,
                0xFFFFFF05u32 as i32,
                0xFFFFFF04u32 as i32,
            );
            let m3: __m256i = _mm256_set_epi32(
                0xFFFFFF0Bu32 as i32,
                0xFFFFFF0Au32 as i32,
                0xFFFFFF09u32 as i32,
                0xFFFFFF08u32 as i32,
                0xFFFFFF0Bu32 as i32,
                0xFFFFFF0Au32 as i32,
                0xFFFFFF09u32 as i32,
                0xFFFFFF08u32 as i32,
            );
            let m4: __m256i = _mm256_set_epi32(
                0xFFFFFF0Fu32 as i32,
                0xFFFFFF0Eu32 as i32,
                0xFFFFFF0Du32 as i32,
                0xFFFFFF0Cu32 as i32,
                0xFFFFFF0Fu32 as i32,
                0xFFFFFF0Eu32 as i32,
                0xFFFFFF0Du32 as i32,
                0xFFFFFF0Cu32 as i32,
            );
            // loop over every row of the sequence data
            for i in 0..seq.data.rows() - seq.wrap {
                let mut s1 = _mm256_setzero_ps();
                let mut s2 = _mm256_setzero_ps();
                let mut s3 = _mm256_setzero_ps();
                let mut s4 = _mm256_setzero_ps();
                for j in 0..pwm.len() {
                    let x = _mm256_load_si256(seq.data[i + j].as_ptr() as *const __m256i);
                    let row = pwm.data[j].as_ptr();
                    // compute probabilities using an external lookup table
                    let p1 = _mm256_i32gather_ps(row, _mm256_shuffle_epi8(x, m1), S);
                    let p2 = _mm256_i32gather_ps(row, _mm256_shuffle_epi8(x, m2), S);
                    let p3 = _mm256_i32gather_ps(row, _mm256_shuffle_epi8(x, m3), S);
                    let p4 = _mm256_i32gather_ps(row, _mm256_shuffle_epi8(x, m4), S);
                    // add log odds
                    s1 = _mm256_add_ps(s1, p1);
                    s2 = _mm256_add_ps(s2, p2);
                    s3 = _mm256_add_ps(s3, p3);
                    s4 = _mm256_add_ps(s4, p4);
                }
                let row = &mut result[i];
                _mm256_store_ps(row[0..].as_mut_ptr(), s1);
                _mm256_store_ps(row[8..].as_mut_ptr(), s2);
                _mm256_store_ps(row[16..].as_mut_ptr(), s3);
                _mm256_store_ps(row[24..].as_mut_ptr(), s4);
            }
        }

        StripedScores {
            length: seq.length - pwm.len() + 1,
            data: result,
            marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StripedScores<V: Vector, const C: usize = 32> {
    pub length: usize,
    pub data: DenseMatrix<f32, C>,
    marker: std::marker::PhantomData<V>,
}

impl<const C: usize> StripedScores<f32, C> {
    pub fn to_vec(&self) -> Vec<f32> {
        let mut vec = Vec::with_capacity(self.length);
        for i in 0..self.length {
            let col = i / self.data.rows();
            let row = i % self.data.rows();
            vec.push(self.data[row][col]);
        }
        vec
    }
}

#[cfg(target_feature = "avx2")]
impl<const C: usize> StripedScores<__m256, C> {
    pub fn to_vec(&self) -> Vec<f32> {
        // NOTE(@althonos): Because in AVX2 the __m256 vector is actually
        //                  two independent __m128, the shuffling creates
        //                  intrication in the results.
        #[rustfmt::skip]
        const COLS: &[usize] = &[
             0,  1,  2,  3,  8,  9, 10, 11, 16, 17, 18, 19, 24, 25, 26, 27,
             4,  5,  6,  7, 12, 13, 14, 15, 20, 21, 22, 23, 28, 29, 30, 31,
        ];

        let mut col = 0;
        let mut row = 0;
        let mut vec = Vec::with_capacity(self.length);
        for i in 0..self.length {
            vec.push(self.data[row][COLS[col]]);
            row += 1;
            if row == self.data.rows() {
                row = 0;
                col += 1;
            }
        }
        vec
    }
}
