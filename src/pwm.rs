use super::abc::Alphabet;
use super::abc::Symbol;
use super::dense::DenseMatrix;
use super::seq::EncodedSequence;
use super::seq::StripedSequence;

#[derive(Clone, Debug)]
pub struct CountMatrix<A: Alphabet, const K: usize> {
    pub alphabet: A,
    pub data: DenseMatrix<u32, K>,
}

impl<A: Alphabet, const K: usize> CountMatrix<A, K> {
    pub fn new(data: DenseMatrix<u32, K>) -> Result<Self, ()> {
        Ok(Self {
            data,
            alphabet: A::default(),
        })
    }

    pub fn from_sequences<'seq, I>(sequences: I) -> Result<Self, ()>
    where
        I: IntoIterator<Item = &'seq EncodedSequence<A>>,
    {
        let mut data = None;
        for seq in sequences {
            let mut d = match data.as_mut() {
                Some(d) => d,
                None => {
                    data = Some(DenseMatrix::new(seq.len()));
                    data.as_mut().unwrap()
                }
            };
            for (i, x) in seq.data.iter().enumerate() {
                d[i][x.as_index()] += 1;
            }
        }

        Ok(Self {
            alphabet: A::default(),
            data: data.unwrap_or_else(|| DenseMatrix::new(0)),
        })
    }

    /// Build a probability matrix from this count matrix using pseudo-counts.
    pub fn to_probability(&self, pseudo: f32) -> ProbabilityMatrix<A, K> {
        let mut probas = DenseMatrix::new(self.data.rows());
        for i in 0..self.data.rows() {
            let src = &self.data[i];
            let mut dst = &mut probas[i];
            for (j, &x) in src.iter().enumerate() {
                dst[j] = x as f32 + pseudo;
            }
            let s: f32 = dst.iter().sum();
            for x in dst.iter_mut() {
                *x /= s;
            }
        }
        ProbabilityMatrix {
            alphabet: self.alphabet,
            data: probas,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProbabilityMatrix<A: Alphabet, const K: usize> {
    pub alphabet: A,
    pub data: DenseMatrix<f32, K>,
}

impl<A: Alphabet, const K: usize> ProbabilityMatrix<A, K> {
    pub fn to_weight(&self, background: Background<A, K>) -> WeightMatrix<A, K> {
        let mut weight = DenseMatrix::new(self.data.rows());
        for i in 0..self.data.rows() {
            let src = &self.data[i];
            let mut dst = &mut weight[i];
            for (j, (&x, &f)) in src.iter().zip(&background.frequencies).enumerate() {
                dst[j] = (x / f).log2();
            }
        }
        WeightMatrix {
            background,
            alphabet: self.alphabet,
            data: weight,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Background<A: Alphabet, const K: usize> {
    pub frequencies: [f32; K],
    _marker: std::marker::PhantomData<A>,
}

impl<A: Alphabet, const K: usize> Background<A, K> {
    pub fn uniform() -> Self {
        Self {
            frequencies: [1.0 / (K as f32); K],
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeightMatrix<A: Alphabet, const K: usize> {
    pub alphabet: A,
    pub background: Background<A, K>,
    pub data: DenseMatrix<f32, K>,
}
