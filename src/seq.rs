use super::abc::Alphabet;
use super::abc::InvalidSymbol;
use super::pwm::WeightMatrix;
use super::dense::DenseMatrix;

#[derive(Clone, Debug)]
pub struct EncodedSequence<A: Alphabet> {
    pub alphabet: A,
    pub data: Vec<A::Symbol>,
}

impl<A: Alphabet> EncodedSequence<A> {
    /// Create a new encoded sequence from a textual representation.
    pub fn from_text(sequence: &str) -> Result<Self, InvalidSymbol>
    where
        InvalidSymbol: From<<A::Symbol as TryFrom<char>>::Error>,
    {
        let data = sequence
            .chars()
            .map(|c| A::Symbol::try_from(c))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            data,
            alphabet: Default::default(),
        })
    }

    /// Return the number of symbols in the sequence.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Convert the encoded sequence to a striped matrix.
    pub fn to_striped<const C: usize>(&self) -> StripedSequence<A, C> {
        let length = self.data.len();
        let n = length / C + ((length % C) != 0) as usize;
        let mut data = DenseMatrix::new(n);
        for (i, &x) in self.data.iter().enumerate() {
            data[i % n][i / n] = x;
        }
        StripedSequence {
            alphabet: self.alphabet,
            data,
            length,
            wrap: 0,
        }
    }
}

impl<A: Alphabet> AsRef<EncodedSequence<A>> for EncodedSequence<A> {
    fn as_ref(&self) -> &Self {
        &self
    }
}

#[derive(Clone, Debug)]
pub struct StripedSequence<A: Alphabet, const C: usize = 32> {
    pub alphabet: A,
    pub length: usize,
    pub wrap: usize,
    pub data: DenseMatrix<A::Symbol, C>,
}

impl<A: Alphabet, const C: usize> StripedSequence<A, C> {

    /// Reconfigure the striped sequence for searching with a motif.
    pub fn configure<const K: usize>(&mut self, motif: &WeightMatrix<A, K>) {
        self.configure_wrap(motif.len());
    }

    /// Add wrap-around rows for a motif of length `m`.
    pub fn configure_wrap(&mut self, m: usize) {
        if m > self.wrap {
            let rows = self.data.rows() - self.wrap;
            self.data.resize( self.data.rows() + m - self.wrap );
            for i in 0..m {
                for j in 0..C-1 {
                    self.data[rows + i][j] = self.data[i][j+1];
                }
            }
            self.wrap = m;
        }
    }
}

impl<A: Alphabet, const C: usize> From<EncodedSequence<A>> for StripedSequence<A, C> {
    fn from(encoded: EncodedSequence<A>) -> Self {
        encoded.to_striped()
    }
}



#[cfg(test)]
mod test {
    use super::*;
    use crate::DnaAlphabet;
    use crate::DnaSymbol::*;

    #[test]
    fn test_stripe() {
        let seq = EncodedSequence::<DnaAlphabet>::from_text("ATGCA").unwrap();
        let striped = seq.to_striped::<4>();
        assert_eq!(striped.data.rows(), 2);
        assert_eq!(&striped.data[0], &[ A, G, A, N ]);
        assert_eq!(&striped.data[1], &[ T, C, N, N ]);

        let striped = seq.to_striped::<2>();
        assert_eq!(striped.data.rows(), 3);
        assert_eq!(&striped.data[0], &[ A, C, ]);
        assert_eq!(&striped.data[1], &[ T, A, ]);
        assert_eq!(&striped.data[2], &[ G, N, ]);
    }

    #[test]
    fn test_configure_wrap() {
        let seq = EncodedSequence::<DnaAlphabet>::from_text("ATGCA").unwrap();
        let mut striped = seq.to_striped::<4>();

        striped.configure_wrap(2);
        assert_eq!(striped.data.rows(), 4);
        assert_eq!(&striped.data[0], &[ A, G, A, N ]);
        assert_eq!(&striped.data[1], &[ T, C, N, N ]);
        assert_eq!(&striped.data[2], &[ G, A, N, N ]);
        assert_eq!(&striped.data[3], &[ C, N, N, N ]);
    }
}