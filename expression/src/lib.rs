/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub mod error;
pub mod eval;
pub mod expr;
mod options;
pub mod parser;
pub mod row;

pub use error::{EvalError, EvalResult};
pub use eval::EvalCell;
pub use expr::Expr;
pub use options::EvalOpts;
pub use row::{ExprRow, TypeRow, ValueRow};
