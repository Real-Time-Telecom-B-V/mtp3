//! PyO3 bindings — `pip install mtp3` gives a Rust-backed wheel exposing the
//! **same** point-code and MTP3-User SAP types the crate ships.
//!
//! Compiled only with `--features python`; the default crate build is pyo3-free,
//! so `cargo add mtp3` / crates.io consumers pull zero pyo3. Two entry points
//! share one `add_contents()`:
//! * `#[pymodule] fn _mtp3` — the standalone wheel (maturin `module-name`).
//! * `pub fn register(py, parent)` — mount `mtp3` as a submodule of another
//!   extension, so a host can expose mtp3 without a second shared object.
//!
//! Scope: the Python surface is the value types — `PointCode` / `Variant`,
//! `ServiceIndicator`, `NetworkIndicator`, and the `Mtp3Msu` SAP-boundary struct.
//! `Mtp3Msu` also carries the MTP3 routing-label codec (`encode` / `decode`, ITU
//! Q.704 and ANSI T1.111), the one wire format this crate owns. The async
//! `Mtp3UserPart` provider trait is deliberately **not** exposed (it is a Rust
//! interface an M2PA/M3UA runtime implements, not a data type).

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use crate::{
    Mtp3Msu as CoreMtp3Msu, NetworkIndicator as CoreNetworkIndicator, PointCode as CorePointCode,
    PointCodeError, ServiceIndicator as CoreServiceIndicator, Variant as CoreVariant,
};

// ── Error mapping ───────────────────────────────────────────────────────────
create_exception!(
    mtp3,
    Mtp3Error,
    PyException,
    "MTP3 point-code / SAP value error."
);

fn pc_err(e: PointCodeError) -> PyErr {
    Mtp3Error::new_err(e.to_string())
}

// ── Variant (ITU / ANSI / China) ────────────────────────────────────────────
/// The SS7 variant — fixes a point code's bit width and structured layout
/// (ITU 14-bit `3-8-3`; ANSI/China 24-bit `8-8-8`).
#[pyclass(name = "Variant", module = "mtp3._mtp3", eq, from_py_object)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyVariant {
    Itu,
    Ansi,
    China,
}

impl PyVariant {
    fn to_core(self) -> CoreVariant {
        match self {
            PyVariant::Itu => CoreVariant::Itu,
            PyVariant::Ansi => CoreVariant::Ansi,
            PyVariant::China => CoreVariant::China,
        }
    }

    fn from_core(v: CoreVariant) -> Self {
        match v {
            CoreVariant::Itu => PyVariant::Itu,
            CoreVariant::Ansi => PyVariant::Ansi,
            CoreVariant::China => PyVariant::China,
        }
    }
}

#[pymethods]
impl PyVariant {
    /// Total point-code width in bits (14 for ITU, 24 for ANSI/China).
    fn bits(&self) -> u8 {
        self.to_core().bits()
    }

    /// Number of octets a point code occupies on the wire (rounded up).
    fn octets(&self) -> usize {
        self.to_core().octets()
    }

    /// Largest value representable in this variant.
    fn max_value(&self) -> u32 {
        self.to_core().max_value()
    }

    /// Component bit widths, most-significant first.
    fn widths(&self) -> [u32; 3] {
        self.to_core().widths().map(u32::from)
    }

    fn __repr__(&self) -> String {
        format!("Variant.{}", self.name())
    }

    fn name(&self) -> &'static str {
        match self {
            PyVariant::Itu => "Itu",
            PyVariant::Ansi => "Ansi",
            PyVariant::China => "China",
        }
    }
}

// ── PointCode ───────────────────────────────────────────────────────────────
/// An SS7 point code plus the [`Variant`] that gives it meaning. Parse from the
/// structured `a-b-c` form or a plain decimal; render back to `a-b-c`.
#[pyclass(
    name = "PointCode",
    module = "mtp3._mtp3",
    eq,
    hash,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyPointCode {
    inner: CorePointCode,
}

#[pymethods]
impl PyPointCode {
    /// Parse the structured form (`"2-1-3"`) or a plain decimal (`"515"`).
    /// Accepts `-` or `.` separators.
    #[staticmethod]
    fn parse(s: &str, variant: PyVariant) -> PyResult<Self> {
        CorePointCode::parse(s, variant.to_core())
            .map(|inner| Self { inner })
            .map_err(pc_err)
    }

    /// Build from a raw integer, validating it fits the variant's width.
    #[staticmethod]
    fn from_value(value: u32, variant: PyVariant) -> PyResult<Self> {
        CorePointCode::from_value(value, variant.to_core())
            .map(|inner| Self { inner })
            .map_err(pc_err)
    }

    /// Build from the three structured components (most-significant first).
    #[staticmethod]
    fn from_components(parts: [u32; 3], variant: PyVariant) -> PyResult<Self> {
        CorePointCode::from_components(parts, variant.to_core())
            .map(|inner| Self { inner })
            .map_err(pc_err)
    }

    /// The raw integer value (as it sits right-aligned in the routing label).
    fn value(&self) -> u32 {
        self.inner.value()
    }

    /// Decompose into the three structured components (most-significant first).
    fn components(&self) -> [u32; 3] {
        self.inner.components()
    }

    /// The variant this point code is interpreted under.
    #[getter]
    fn variant(&self) -> PyVariant {
        PyVariant::from_core(self.inner.variant())
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "PointCode('{}', Variant.{})",
            self.inner,
            PyVariant::from_core(self.inner.variant()).name()
        )
    }
}

// ── ServiceIndicator ────────────────────────────────────────────────────────
/// Service Indicator — the low nibble of the SIO, naming the MTP3-user that owns
/// the message. An opaque `u8`; the well-known values have class constants.
#[pyclass(
    name = "ServiceIndicator",
    module = "mtp3._mtp3",
    eq,
    hash,
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyServiceIndicator {
    inner: CoreServiceIndicator,
}

#[pymethods]
impl PyServiceIndicator {
    #[new]
    fn new(value: u8) -> Self {
        Self {
            inner: CoreServiceIndicator(value),
        }
    }

    /// The raw 4-bit value.
    #[getter]
    fn value(&self) -> u8 {
        self.inner.0
    }

    /// The well-known name (e.g. `"SCCP"`), or `None`.
    fn name(&self) -> Option<&'static str> {
        self.inner.name()
    }

    // Well-known Service Indicators.
    #[classattr]
    #[allow(non_snake_case)]
    fn SNM() -> Self {
        Self {
            inner: CoreServiceIndicator::SNM,
        }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn SNT() -> Self {
        Self {
            inner: CoreServiceIndicator::SNT,
        }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn SCCP() -> Self {
        Self {
            inner: CoreServiceIndicator::SCCP,
        }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn TUP() -> Self {
        Self {
            inner: CoreServiceIndicator::TUP,
        }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn ISUP() -> Self {
        Self {
            inner: CoreServiceIndicator::ISUP,
        }
    }

    fn __int__(&self) -> u8 {
        self.inner.0
    }

    fn __repr__(&self) -> String {
        match self.inner.name() {
            Some(n) => format!("ServiceIndicator({}, {n})", self.inner.0),
            None => format!("ServiceIndicator({})", self.inner.0),
        }
    }
}

// ── NetworkIndicator ────────────────────────────────────────────────────────
/// Network Indicator — the top two bits of the SIO. Integer values are the
/// on-wire encoding (`International == 0`, `National == 2`).
#[pyclass(
    name = "NetworkIndicator",
    module = "mtp3._mtp3",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyNetworkIndicator {
    International = 0,
    InternationalSpare = 1,
    National = 2,
    NationalSpare = 3,
}

impl PyNetworkIndicator {
    fn to_core(self) -> CoreNetworkIndicator {
        match self {
            PyNetworkIndicator::International => CoreNetworkIndicator::International,
            PyNetworkIndicator::InternationalSpare => CoreNetworkIndicator::InternationalSpare,
            PyNetworkIndicator::National => CoreNetworkIndicator::National,
            PyNetworkIndicator::NationalSpare => CoreNetworkIndicator::NationalSpare,
        }
    }

    fn from_core(n: CoreNetworkIndicator) -> Self {
        match n {
            CoreNetworkIndicator::International => PyNetworkIndicator::International,
            CoreNetworkIndicator::InternationalSpare => PyNetworkIndicator::InternationalSpare,
            CoreNetworkIndicator::National => PyNetworkIndicator::National,
            CoreNetworkIndicator::NationalSpare => PyNetworkIndicator::NationalSpare,
        }
    }
}

#[pymethods]
impl PyNetworkIndicator {
    /// Build from the raw two-bit SIO field (masked to 2 bits).
    #[staticmethod]
    fn from_bits(v: u8) -> Self {
        Self::from_core(CoreNetworkIndicator::from_bits(v))
    }

    /// The two-bit SIO encoding.
    fn bits(&self) -> u8 {
        self.to_core().bits()
    }
}

// ── Mtp3Msu ─────────────────────────────────────────────────────────────────
/// An MTP3 Message Signal Unit as seen at the MTP3-user boundary — the routing
/// label (OPC/DPC/SLS), the SIO fields (SI/NI/priority), and the user payload.
/// The parameters of the MTP-TRANSFER primitive (Q.701).
///
/// `encode` / `decode` render it to and from the on-wire MSU (SIO + routing
/// label + SIF) for a given [`Variant`] — ITU Q.704 or ANSI T1.111.
#[pyclass(name = "Mtp3Msu", module = "mtp3._mtp3", skip_from_py_object)]
#[derive(Clone)]
pub struct PyMtp3Msu {
    #[pyo3(get, set)]
    pub si: PyServiceIndicator,
    #[pyo3(get, set)]
    pub ni: PyNetworkIndicator,
    #[pyo3(get, set)]
    pub mp: u8,
    #[pyo3(get, set)]
    pub opc: PyPointCode,
    #[pyo3(get, set)]
    pub dpc: PyPointCode,
    #[pyo3(get, set)]
    pub sls: u8,
    data: Vec<u8>,
}

impl PyMtp3Msu {
    fn to_core(&self) -> CoreMtp3Msu {
        CoreMtp3Msu {
            si: self.si.inner,
            ni: self.ni.to_core(),
            mp: self.mp,
            opc: self.opc.inner,
            dpc: self.dpc.inner,
            sls: self.sls,
            data: self.data.clone(),
        }
    }

    fn from_core(m: CoreMtp3Msu) -> Self {
        Self {
            si: PyServiceIndicator { inner: m.si },
            ni: PyNetworkIndicator::from_core(m.ni),
            mp: m.mp,
            opc: PyPointCode { inner: m.opc },
            dpc: PyPointCode { inner: m.dpc },
            sls: m.sls,
            data: m.data,
        }
    }
}

#[pymethods]
impl PyMtp3Msu {
    #[new]
    #[pyo3(signature = (si, ni, opc, dpc, *, mp = 0, sls = 0, data = Vec::new()))]
    fn new(
        si: PyServiceIndicator,
        ni: PyNetworkIndicator,
        opc: PyPointCode,
        dpc: PyPointCode,
        mp: u8,
        sls: u8,
        data: Vec<u8>,
    ) -> Self {
        Self {
            si,
            ni,
            // Core masks priority to its 2 valid bits; mirror that here.
            mp: mp & 0x03,
            opc,
            dpc,
            sls,
            data,
        }
    }

    /// The MTP3-user payload (e.g. an encoded SCCP message) as `bytes`.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    #[setter]
    fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    /// Encode to the on-wire MSU bytes for `variant`: SIO octet + routing label
    /// + SIF. ITU packs a 32-bit little-endian label; ANSI/China lay it out as
    /// DPC(3) + OPC(3) + SLS(1) octets (T1.111).
    fn encode<'py>(&self, py: Python<'py>, variant: PyVariant) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.to_core().encode(variant.to_core()))
    }

    /// Decode on-wire MSU bytes into an `Mtp3Msu` under `variant`. Raises
    /// `Mtp3Error` if the input is too short for the SIO + routing label.
    #[staticmethod]
    fn decode(data: Vec<u8>, variant: PyVariant) -> PyResult<Self> {
        CoreMtp3Msu::decode(&data, variant.to_core())
            .map(Self::from_core)
            .map_err(|e| Mtp3Error::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Mtp3Msu(si={}, ni={}, opc='{}', dpc='{}', mp={}, sls={}, data_len={})",
            self.si.__repr__(),
            self.ni as u8,
            self.opc.inner,
            self.dpc.inner,
            self.mp,
            self.sls,
            self.data.len()
        )
    }
}

// ── Module wiring ───────────────────────────────────────────────────────────
fn add_contents(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("Mtp3Error", m.py().get_type::<Mtp3Error>())?;
    m.add_class::<PyVariant>()?;
    m.add_class::<PyPointCode>()?;
    m.add_class::<PyServiceIndicator>()?;
    m.add_class::<PyNetworkIndicator>()?;
    m.add_class::<PyMtp3Msu>()?;
    Ok(())
}

/// Standalone wheel entry point (maturin `module-name = "mtp3._mtp3"`).
#[pymodule]
fn _mtp3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    add_contents(m)
}

/// Embedding entry point: build an `mtp3` submodule and attach it to `parent`,
/// so a host extension can expose mtp3 without a second shared object.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "mtp3")?;
    add_contents(&m)?;
    parent.setattr("mtp3", &m)?;
    Ok(())
}
