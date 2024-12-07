use crate::trnsys::error::TrnSysError;
use crate::trnsys::get_lu_filename;
use std::panic::catch_unwind;

pub struct TrnSysValue {
    pub value: f64,
}

impl TrnSysValue {
    pub fn new(value: f64) -> Self {
        TrnSysValue { value }
    }
}

impl From<f64> for TrnSysValue {
    fn from(value: f64) -> Self {
        TrnSysValue { value }
    }
}

impl From<i32> for TrnSysValue {
    fn from(value: i32) -> Self {
        TrnSysValue {
            value: value as f64,
        }
    }
}

impl From<&TrnSysValue> for f64 {
    fn from(param: &TrnSysValue) -> Self {
        param.value
    }
}

impl TryFrom<&TrnSysValue> for i32 {
    type Error = TrnSysError;
    fn try_from(value: &TrnSysValue) -> Result<Self, Self::Error> {
        if value.value.is_nan() {
            Err(TrnSysError::ConversionError {
                param: "TrnSysValue".to_string(),
                message: "Cannot convert NaN to i32".to_string(),
            })
        } else if value.value.fract() != 0.0 {
            Err(TrnSysError::ConversionError {
                param: "TrnSysValue".to_string(),
                message: "Cannot convert float to i32".to_string(),
            })
        } else {
            Ok(value.value as i32)
        }
    }
}

impl TryFrom<&TrnSysValue> for String {
    type Error = String;

    fn try_from(param: &TrnSysValue) -> Result<Self, Self::Error> {
        // try to convert the float to the string
        // by reading the logical unit of the file from value
        let logical_unit = (param.value + 0.1) as i32;
        let fname = catch_unwind(|| get_lu_filename(logical_unit));

        match fname {
            Ok(name) => Ok(name),
            Err(e) => match e.downcast::<String>() {
                Ok(panic_msg) => {
                    let panic_msg = format!("Cannot read param as string:{}", panic_msg);
                    println!("{}", &panic_msg);
                    Err(panic_msg.to_string())
                }
                Err(_) => {
                    let panic_msg = "Cannot read param as string: unknown error";
                    println!("{}", &panic_msg);
                    Err(panic_msg.to_string())
                }
            },
        }
    }
}
