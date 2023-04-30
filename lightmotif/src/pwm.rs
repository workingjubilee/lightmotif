use super::abc::Alphabet;
use super::abc::Symbol;
use super::dense::DenseMatrix;
use super::seq::EncodedSequence;

/// A structure for storing the pseudocounts over an alphabet.
#[derive(Clone, Debug)]
pub struct Pseudocount<A: Alphabet, const K: usize> {
    alphabet: std::marker::PhantomData<A>,
    pub counts: [f32; K],
}

impl<A: Alphabet, const K: usize> From<[f32; K]> for Pseudocount<A, K> {
    fn from(counts: [f32; K]) -> Self {
        Self {
            alphabet: std::marker::PhantomData,
            counts,
        }
    }
}

impl<A: Alphabet, const K: usize> From<f32> for Pseudocount<A, K> {
    fn from(count: f32) -> Self {
        let mut counts: [f32; K] = [0.0; K];
        for i in 0..K {
            if i != A::default_symbol().as_index() {
                counts[i] = count;
            }
        }
        Self {
            counts,
            alphabet: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CountMatrix<A: Alphabet, const K: usize> {
    alphabet: std::marker::PhantomData<A>,
    pub data: DenseMatrix<u32, K>,
}

impl<A: Alphabet, const K: usize> CountMatrix<A, K> {
    pub fn new(data: DenseMatrix<u32, K>) -> Result<Self, ()> {
        Ok(Self {
            data,
            alphabet: std::marker::PhantomData,
        })
    }

    pub fn from_sequences<'seq, I>(sequences: I) -> Result<Self, ()>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: AsRef<EncodedSequence<A>>,
    {
        let mut data = None;
        for seq in sequences {
            let seq = seq.as_ref();
            let d = match data.as_mut() {
                Some(d) => d,
                None => {
                    data = Some(DenseMatrix::new(seq.len()));
                    data.as_mut().unwrap()
                }
            };
            if seq.len() != d.rows() {
                return Err(());
            }
            for (i, x) in seq.data.iter().enumerate() {
                d[i][x.as_index()] += 1;
            }
        }

        Ok(Self {
            alphabet: std::marker::PhantomData,
            data: data.unwrap_or_else(|| DenseMatrix::new(0)),
        })
    }

    /// Build a probability matrix from this count matrix using pseudo-counts.
    pub fn to_probability<P>(&self, pseudo: P) -> ProbabilityMatrix<A, K>
    where
        P: Into<Pseudocount<A, K>>,
    {
        let p = pseudo.into();
        let mut probas = DenseMatrix::new(self.data.rows());
        for i in 0..self.data.rows() {
            let src = &self.data[i];
            let dst = &mut probas[i];
            for (j, &x) in src.iter().enumerate() {
                dst[j] = x as f32 + p.counts[j] as f32;
            }
            let s: f32 = dst.iter().sum();
            for x in dst.iter_mut() {
                *x /= s;
            }
        }
        ProbabilityMatrix {
            alphabet: std::marker::PhantomData,
            data: probas,
        }
    }
}

impl<A: Alphabet, const K: usize> AsRef<DenseMatrix<u32, K>> for CountMatrix<A, K> {
    fn as_ref(&self) -> &DenseMatrix<u32, K> {
        &self.data
    }
}

impl<A: Alphabet, const K: usize> AsMut<DenseMatrix<u32, K>> for CountMatrix<A, K> {
    fn as_mut(&mut self) -> &mut DenseMatrix<u32, K> {
        &mut self.data
    }
}

#[derive(Clone, Debug)]
pub struct ProbabilityMatrix<A: Alphabet, const K: usize> {
    alphabet: std::marker::PhantomData<A>,
    pub data: DenseMatrix<f32, K>,
}

impl<A: Alphabet, const K: usize> ProbabilityMatrix<A, K> {
    pub fn to_weight<B>(&self, background: B) -> WeightMatrix<A, K>
    where
        B: Into<Background<A, K>>,
    {
        let b = background.into();
        let mut weight = DenseMatrix::new(self.data.rows());
        for i in 0..self.data.rows() {
            let src = &self.data[i];
            let dst = &mut weight[i];
            for (j, (&x, &f)) in src.iter().zip(&b.frequencies).enumerate() {
                dst[j] = (x / f).log2();
            }
        }
        WeightMatrix {
            background: b,
            alphabet: std::marker::PhantomData,
            data: weight,
        }
    }
}

impl<A: Alphabet, const K: usize> AsRef<DenseMatrix<f32, K>> for ProbabilityMatrix<A, K> {
    fn as_ref(&self) -> &DenseMatrix<f32, K> {
        &self.data
    }
}

impl<A: Alphabet, const K: usize> AsMut<DenseMatrix<f32, K>> for ProbabilityMatrix<A, K> {
    fn as_mut(&mut self) -> &mut DenseMatrix<f32, K> {
        &mut self.data
    }
}

#[derive(Clone, Debug)]
pub struct Background<A: Alphabet, const K: usize> {
    pub frequencies: [f32; K],
    alphabet: std::marker::PhantomData<A>,
}

impl<A: Alphabet, const K: usize> Background<A, K> {
    pub fn uniform() -> Self {

        let mut frequencies = [0.0; K];
        for i in 0..K {
            if i != A::default_symbol().as_index() {
                frequencies[i] = 1.0 / ((K-1) as f32);
            }
        }

        Self {
            frequencies,
            alphabet: std::marker::PhantomData,
        }
    }
}

impl<A: Alphabet, const K: usize> From<[f32; K]> for Background<A, K> {
    fn from(frequencies: [f32; K]) -> Self {
        Self {
            frequencies,
            alphabet: std::marker::PhantomData,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WeightMatrix<A: Alphabet, const K: usize> {
    alphabet: std::marker::PhantomData<A>,
    pub background: Background<A, K>,
    pub data: DenseMatrix<f32, K>,
}

impl<A: Alphabet, const K: usize> WeightMatrix<A, K> {
    /// The length of the motif encoded in this weight matrix.
    pub fn len(&self) -> usize {
        self.data.rows()
    }
}

impl<A: Alphabet, const K: usize> AsRef<WeightMatrix<A, K>> for WeightMatrix<A, K> {
    fn as_ref(&self) -> &Self {
        &self
    }
}

impl<A: Alphabet, const K: usize> AsRef<DenseMatrix<f32, K>> for WeightMatrix<A, K> {
    fn as_ref(&self) -> &DenseMatrix<f32, K> {
        &self.data
    }
}

impl<A: Alphabet, const K: usize> AsMut<DenseMatrix<f32, K>> for WeightMatrix<A, K> {
    fn as_mut(&mut self) -> &mut DenseMatrix<f32, K> {
        &mut self.data
    }
}
