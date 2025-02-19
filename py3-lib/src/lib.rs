/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use pyo3::{pyclass, pymethods, pymodule, types::PyModule, PyResult, Python};
use serde::Serialize;

#[pymodule]
/// Parse and evaluate smart agent expressions.
fn smart_agent(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Expr>()?;
    Ok(())
}

#[pyclass]
#[derive(Debug, Clone)]
struct Expr(expression::Expr);

#[pymethods]
impl Expr {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        let expr = expression::parser::parse_expr(s).map_err(|e| {
            pyo3::exceptions::PyException::new_err(e.to_string())
        })?;
        Ok(Self(expr))
    }

    // fn data(&self) -> PyResult<PyObject> {
    //     Ok(self.0.into())
    // }

    fn dumps(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyTypeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(self.0.py_repr().to_string())
    }
}

impl Serialize for Expr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
