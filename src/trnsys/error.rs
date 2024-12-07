use crate::trnsys::{found_bad_input, found_bad_parameter, Severity, TrnSysState};
use thiserror::Error;
use tracing::error;

pub trait TrnSysErrorHandler {
    fn handle_in_trnsys(&self, state: &TrnSysState);
}

#[derive(Error, Debug)]
pub enum TrnSysError {
    #[error("Error in TrnSys Type: {0}")]
    GeneralError(String),
    #[error("Error in TrnSys: {0}")]
    InputError(#[from] InputError),
    #[error("Cannot convert {param}: {message}")]
    ConversionError { param: String, message: String },
}

impl TrnSysErrorHandler for TrnSysError {
    fn handle_in_trnsys(&self, state: &TrnSysState) {
        match self {
            TrnSysError::InputError(e) => {
                e.handle_in_trnsys(state);
            }
            _ => {}
        }
    }
}

/// Input Error for TRNSYS
#[derive(Error, Debug)]
pub enum InputError {
    #[error("Bad Input at {index}: {message}")]
    BadInput { index: i32, message: String },
    #[error("Bad Parameter at {index}: {message}")]
    BadParameter { index: i32, message: String },
}

impl TrnSysErrorHandler for InputError {
    fn handle_in_trnsys(&self, state: &TrnSysState) {
        match self {
            InputError::BadInput { index, message } => {
                found_bad_input(index.clone(), Severity::Fatal, &format!("{:?}", self));
            }
            InputError::BadParameter { index, message } => {
                found_bad_parameter(index.clone(), Severity::Fatal, &format!("{:?}", self));
            }
        }
    }
}
