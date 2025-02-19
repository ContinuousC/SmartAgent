/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use cpython::{
    py_class, py_fn, py_module_initializer, PyErr, PyObject, PyResult,
    PyString, Python,
};

py_module_initializer!(smart_agent, |py, m| {
    m.add(py, "__doc__", "Parse and evaluate smart agent expressions.")?;
    m.add(py, "parse_expr", py_fn!(py, parse_expr(s: &str)))?;
    m.add_class::<Expr>(py)?;
    Ok(())
});

py_class!(class Expr |py| {
    data inner: expression::Expr;
    def __new__(_cls, s: &str) -> PyResult<Self> {
        Self::create_instance(py, expression::parser::parse_expr(s).map_err(|e| {
            PyErr::new::<cpython::exc::Exception, _>(
                py,
                PyString::new(py, &e.to_string()),
            )
        })?)
    }
    def data(&self) -> PyResult<PyObject> {
        cpython::serde::to_py_object(py, self.inner(py))
    }
    def __repr__(&self) -> PyResult<String> {
        Ok(self.inner(py).py_repr().to_string())
    }
});

fn parse_expr(py: Python<'_>, s: &str) -> PyResult<PyObject> {
    cpython::serde::to_py_object(
        py,
        &expression::parser::parse_expr(s).map_err(|e| {
            PyErr::new::<cpython::exc::Exception, _>(
                py,
                PyString::new(py, &e.to_string()),
            )
        })?,
    )
}
