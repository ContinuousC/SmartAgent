/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/



pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    
}

pub type DTEResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {
    
}
