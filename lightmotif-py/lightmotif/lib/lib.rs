extern crate pyo3;
#[macro_use]
extern crate pyo3_built;
extern crate lightmotif;

#[cfg(target_feature = "avx2")]
use std::arch::x86_64::{__m256, __m256i};

use lightmotif as lm;
use lightmotif::Alphabet;
use lightmotif::Pipeline;
use lightmotif::Score;
use lightmotif::Symbol;

use pyo3::exceptions::PyBufferError;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::ffi::Py_ssize_t;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyString;
use pyo3::AsPyPointer;

#[allow(dead_code)]
mod build {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

// --- Compile-time constants --------------------------------------------------

#[cfg(target_feature = "avx2")]
type Vector = __m256;
#[cfg(target_feature = "avx2")]
const C: usize = std::mem::size_of::<__m256i>();

#[cfg(not(target_feature = "avx2"))]
type Vector = f32;
#[cfg(not(target_feature = "avx2"))]
const C: usize = std::mem::size_of::<f32>();

// --- Helpers -----------------------------------------------------------------

fn dict_to_alphabet_array<'py, A: lm::Alphabet, const K: usize>(
    d: &'py PyDict,
) -> PyResult<[f32; K]> {
    let mut p = [0.0; K];
    for (k, v) in d.iter() {
        let s = k.extract::<&PyString>()?.to_str()?;
        if s.len() != 1 {
            return Err(PyValueError::new_err((
                "Invalid key for pseudocount:",
                s.to_string(),
            )));
        }
        let x = s.chars().next().unwrap();
        let symbol = <A as lm::Alphabet>::Symbol::from_char(x)
            .map_err(|_| PyValueError::new_err(("Invalid key for pseudocount:", x)))?;
        let value = v.extract::<f32>()?;
        p[symbol.as_index()] = value;
    }
    Ok(p)
}

// --- EncodedSequence ---------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct EncodedSequence {
    data: lm::EncodedSequence<lm::Dna>,
}

#[pymethods]
impl EncodedSequence {
    /// Encode a sequence with the given alphabet.
    #[new]
    pub fn __init__(sequence: &PyString) -> PyResult<PyClassInitializer<Self>> {
        let seq = sequence.to_str()?;
        let data = lm::EncodedSequence::encode(&seq).map_err(|lm::InvalidSymbol(x)| {
            PyValueError::new_err(format!("Invalid symbol in input: {}", x))
        })?;
        Ok(EncodedSequence { data }.into())
    }

    /// Create a copy of this sequence.
    pub fn copy(&self) -> EncodedSequence {
        self.clone()
    }

    /// Convert this sequence into a striped matrix.
    pub fn stripe(&self) -> StripedSequence {
        StripedSequence {
            data: self.data.to_striped(),
        }
    }
}

// --- StripedSequence ---------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct StripedSequence {
    data: lm::StripedSequence<lm::Dna, C>,
}

// --- CountMatrix -------------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct CountMatrix {
    data: lm::CountMatrix<lm::Dna, { lm::Dna::K }>,
}

#[pymethods]
impl CountMatrix {
    pub fn normalize(&self, pseudocount: Option<PyObject>) -> PyResult<FrequencyMatrix> {
        let pseudo = Python::with_gil(|py| {
            if let Some(obj) = pseudocount {
                if let Ok(x) = obj.extract::<f32>(py) {
                    Ok(lm::Pseudocounts::from(x))
                } else if let Ok(d) = obj.extract::<&PyDict>(py) {
                    let p = dict_to_alphabet_array::<lm::Dna, { lm::Dna::K }>(d)?;
                    Ok(lm::Pseudocounts::from(p))
                } else {
                    Err(PyTypeError::new_err("Invalid type for pseudocount"))
                }
            } else {
                Ok(lm::Pseudocounts::default())
            }
        })?;
        let data = self.data.to_freq(pseudo);
        Ok(FrequencyMatrix { data })
    }
}

// --- FrequencyMatrix ---------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct FrequencyMatrix {
    data: lm::FrequencyMatrix<lm::Dna, { lm::Dna::K }>,
}

#[pymethods]
impl FrequencyMatrix {
    pub fn log_odds(&self, background: Option<PyObject>) -> PyResult<ScoringMatrix> {
        let bg = Python::with_gil(|py| {
            if let Some(obj) = background {
                if let Ok(d) = obj.extract::<&PyDict>(py) {
                    let p = dict_to_alphabet_array::<lm::Dna, { lm::Dna::K }>(d)?;
                    lm::Background::new(p)
                        .map_err(|_| PyValueError::new_err("Invalid background frequencies"))
                } else {
                    Err(PyTypeError::new_err("Invalid type for pseudocount"))
                }
            } else {
                Ok(lm::Background::uniform())
            }
        })?;
        let data = self.data.to_scoring(bg);
        Ok(ScoringMatrix { data })
    }
}

// --- ScoringMatrix -----------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct ScoringMatrix {
    data: lm::ScoringMatrix<lm::Dna, { lm::Dna::K }>,
}

#[pymethods]
impl ScoringMatrix {
    /// Return the PSSM score for all positions of the given sequence.
    pub fn calculate(
        slf: PyRef<'_, Self>,
        sequence: &mut StripedSequence,
    ) -> PyResult<StripedScores> {
        let pssm = &slf.data;
        sequence.data.configure(pssm);
        let scores = slf
            .py()
            .allow_threads(|| Pipeline::<lm::Dna, Vector>::score(&sequence.data, pssm));
        Ok(StripedScores::from(scores))
    }
}

// --- Scores ------------------------------------------------------------------

#[pyclass(module = "lightmotif.lib")]
#[derive(Clone, Debug)]
pub struct StripedScores {
    scores: lm::StripedScores<Vector, C>,
    shape: [Py_ssize_t; 2],
    strides: [Py_ssize_t; 2],
}

#[pymethods]
impl StripedScores {
    fn __len__(&self) -> usize {
        self.scores.len()
    }

    fn __getitem__(&self, index: isize) -> PyResult<f32> {
        if index < self.scores.len() as isize && index >= 0 {
            Ok(self.scores[index as usize])
        } else {
            Err(PyIndexError::new_err("list index out of range"))
        }
    }

    unsafe fn __getbuffer__(
        mut slf: PyRefMut<'_, Self>,
        view: *mut pyo3::ffi::Py_buffer,
        flags: std::os::raw::c_int,
    ) -> PyResult<()> {
        if view.is_null() {
            return Err(PyBufferError::new_err("View is null"));
        }
        if (flags & pyo3::ffi::PyBUF_WRITABLE) == pyo3::ffi::PyBUF_WRITABLE {
            return Err(PyBufferError::new_err("Object is not writable"));
        }

        (*view).obj = pyo3::ffi::_Py_NewRef(slf.as_ptr());

        let data = slf.scores.matrix()[0].as_ptr();

        (*view).buf = data as *mut std::os::raw::c_void;
        (*view).len = slf.scores.len() as isize;
        (*view).readonly = 1;
        (*view).itemsize = std::mem::size_of::<f32>() as isize;

        let msg = std::ffi::CStr::from_bytes_with_nul(b"f\0").unwrap();
        (*view).format = msg.as_ptr() as *mut _;

        (*view).ndim = 2;
        (*view).shape = slf.shape.as_mut_ptr();
        (*view).strides = slf.strides.as_mut_ptr();

        (*view).suboffsets = std::ptr::null_mut();
        (*view).internal = std::ptr::null_mut();

        Ok(())
    }
}

impl From<lm::StripedScores<Vector, C>> for StripedScores {
    fn from(mut scores: lm::StripedScores<Vector, C>) -> Self {
        // extract the matrix shape
        let cols = scores.matrix().columns();
        let rows = scores.matrix().rows();
        // record the matrix shape as a Fortran buffer
        let shape = [cols as Py_ssize_t, rows as Py_ssize_t];
        let strides = [
            std::mem::size_of::<f32>() as Py_ssize_t,
            (cols.next_power_of_two() * std::mem::size_of::<f32>()) as Py_ssize_t,
        ];
        // mask the remaining positions that are outside the sequence length
        let length = scores.len();
        let data = scores.matrix_mut();
        for i in length..rows * cols {
            let row = i % rows;
            let col = i / rows;
            data[row][col] = -f32::INFINITY;
        }
        // return a Python object implementing the buffer protocol
        Self {
            scores,
            shape,
            strides,
        }
    }
}

// --- Module ------------------------------------------------------------------

#[pyfunction]
fn create<'py>(sequences: &'py PyAny) -> PyResult<CountMatrix> {
    let py = sequences.py();

    let mut encoded = Vec::new();
    for seq in sequences.iter()? {
        let s = seq?.extract::<&PyString>()?.to_str()?;
        let x = py
            .allow_threads(|| lm::EncodedSequence::encode(&s))
            .map_err(|_| PyValueError::new_err("Invalid symbol in sequence"))?;
        encoded.push(x);
    }

    let data = lm::CountMatrix::from_sequences(encoded)
        .map_err(|_| PyValueError::new_err("Inconsistent sequence length"))?;
    Ok(CountMatrix { data })
}

/// PyO3 bindings to ``lightmotif``, a library for fast PWM motif scanning.
///
/// The API is similar to the `Bio.motifs` module from Biopython on purpose.
#[pymodule]
#[pyo3(name = "lib")]
pub fn init(py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__package__", "pyskani")?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", env!("CARGO_PKG_AUTHORS").replace(':', "\n"))?;
    m.add("__build__", pyo3_built!(py, build))?;

    m.add_class::<EncodedSequence>()?;
    m.add_class::<StripedSequence>()?;

    m.add_class::<CountMatrix>()?;
    m.add_class::<FrequencyMatrix>()?;
    m.add_class::<ScoringMatrix>()?;

    m.add_function(wrap_pyfunction!(create, m)?)?;

    Ok(())
}
