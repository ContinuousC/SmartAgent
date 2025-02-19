/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

use value::{Data, Type, Value};

use super::error::EvalError;
use super::eval::EvalCell;
use super::expr::Expr;
use super::options::EvalOpts;

#[derive(Clone, Debug)]
pub struct ExprRow<'a>(pub LinkedHashMap<&'a str, Expr>);

#[derive(Clone, Debug)]
pub struct TypeRow<'a>(pub LinkedHashMap<&'a str, Result<Type, EvalError>>);

#[derive(Clone, Debug)]
pub struct ValueRow<'a>(pub LinkedHashMap<&'a str, Result<Value, EvalError>>);

impl<'a> ExprRow<'a> {
    pub fn eval(&self, data: HashMap<&'a str, Data>) -> ValueRow<'a> {
        self.eval_opts(data, &EvalOpts::default())
    }

    pub fn eval_opts(
        &self,
        data: HashMap<&'a str, Data>,
        opts: &EvalOpts,
    ) -> ValueRow<'a> {
        let eval_vars: HashMap<_, _> = self
            .0
            .iter()
            .map(|(n, e)| (*n, EvalCell::new(e, data.get(n).cloned())))
            .collect();

        ValueRow(
            self.0
                .iter()
                .map(|(n, _)| {
                    (
                        *n,
                        eval_vars[n].eval(|e, d| {
                            e.eval_in_row_opts(Some(&eval_vars), d, opts)
                        }),
                    )
                })
                .collect(),
        )
    }

    pub fn check(&self, data: HashMap<&'a str, Type>) -> TypeRow<'a> {
        self.check_opts(data, &EvalOpts::default())
    }

    pub fn check_opts(
        &self,
        data: HashMap<&'a str, Type>,
        opts: &EvalOpts,
    ) -> TypeRow<'a> {
        let eval_vars: HashMap<_, _> = self
            .0
            .iter()
            .map(|(n, e)| (*n, EvalCell::new(e, data.get(n).cloned())))
            .collect();

        TypeRow(
            self.0
                .iter()
                .map(|(n, _)| {
                    (
                        *n,
                        eval_vars[n].eval(|e, d| {
                            e.check_in_row_opts(Some(&eval_vars), d, opts)
                        }),
                    )
                })
                .collect(),
        )
    }
}
