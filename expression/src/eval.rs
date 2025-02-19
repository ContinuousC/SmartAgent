/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::error::EvalError;
use super::expr::Expr;
use std::cell::Cell;

#[derive(Clone, Debug)]
pub(super) enum Eval<'a, T, R> {
    Expr(&'a Expr, Option<T>),
    Done(Result<R, EvalError>),
    Evaluating,
}

pub struct EvalCell<'a, T, R>(Cell<Eval<'a, T, R>>);

impl<'a, T, R: Clone> EvalCell<'a, T, R> {
    pub fn new(expr: &'a Expr, data: Option<T>) -> Self {
        EvalCell(Cell::new(Eval::Expr(expr, data)))
    }

    pub fn new_evaluated(value: Result<R, EvalError>) -> Self {
        EvalCell(Cell::new(Eval::Done(value)))
    }

    pub fn eval<F>(&self, fun: F) -> Result<R, EvalError>
    where
        F: FnOnce(&Expr, Option<&T>) -> Result<R, EvalError>,
    {
        match self.0.replace(Eval::Evaluating) {
            Eval::Expr(e, d) => {
                let val = (fun)(e, d.as_ref());
                self.0.set(Eval::Done(val.clone()));
                val
            }
            Eval::Done(val) => {
                self.0.set(Eval::Done(val.clone()));
                val
            }
            Eval::Evaluating => Err(EvalError::RecursionError),
        }
    }
}
