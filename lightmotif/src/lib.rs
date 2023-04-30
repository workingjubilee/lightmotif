//! A lightweight platform-accelerated library for biological motif scanning using position weight matrices.

mod abc;
mod dense;
mod pli;
mod pwm;
mod seq;

pub use abc::Alphabet;
pub use abc::DnaAlphabet;
pub use abc::DnaSymbol;
pub use abc::Symbol;
pub use dense::DenseMatrix;
pub use pli::Pipeline;
pub use pli::StripedScores;
pub use pwm::Background;
pub use pwm::CountMatrix;
pub use pwm::ProbabilityMatrix;
pub use pwm::WeightMatrix;
pub use seq::EncodedSequence;
pub use seq::StripedSequence;
