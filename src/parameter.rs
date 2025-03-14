use crate::database::column::MetaCol;
use crate::database::odbc::OdbcProvider;
use crate::trnsys::error::{InputError, TrnSysError};
use crate::trnsys::TrnSysState;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use tracing::{debug, info};

#[derive(Debug, PartialEq, Clone, Copy, TryFromPrimitive, IntoPrimitive)]
#[repr(i32)]
pub enum DriverMode {
    MsAccessFile = 1,
    MsExcelFile = 2,
    SqliteFile = 3,
    ConnectionString = 4,
    Postgres = 5,
}

#[derive(Debug, Clone)]
pub struct Parameters {
    pub print_interval: f64,
    pub driver_mode: DriverMode,
    pub num_inputs: i32,
    pub connection_string: String,
    pub table_name: String,
    pub input_names: Vec<String>,
    pub variant_name: String,
    pub primary_keys: Vec<MetaCol>,
}

impl TryFrom<&TrnSysState> for Parameters {
    type Error = TrnSysError;

    fn try_from(state: &TrnSysState) -> Result<Self, Self::Error> {
        let value = &state.params;
        if value.len() != 3 {
            return Err(InputError::BadParameter {
                index: 0,
                message: "Exact 3 parameters must be given.".to_string(),
            }
            .into());
        }
        let print_interval = value[0].value;
        let driver_mode = DriverMode::try_from(i32::try_from(&value[1])?).map_err(|e| {
            TrnSysError::ConversionError {
                param: "DriverMode".to_string(),
                message: e.to_string(),
            }
        })?;
        let num_inputs = i32::try_from(&value[2])?;

        // Get string from labels
        if state.num_labels < 3 {
            return Err(InputError::BadParameter {
                index: 2,
                message: format!(
                    "At least 3 labels must be given. Got {} labels.",
                    state.num_labels
                ),
            }
            .into());
        }

        let connection_string = state.labels[0].clone();

        let table_name = state.labels[1].clone();

        let variant_name = state.labels[2].clone();

        let input_names: Vec<String> = state.labels[3..].to_vec();

        debug!("input_names: {:?}", input_names);

        if input_names.len() != num_inputs as usize {
            return Err(InputError::BadParameter {
                index: 2,
                message: format!("Number of input names does not match the number of inputs. Expected {}, got {}.", num_inputs, input_names.len()),
            }.into());
        }

        let parameters = Parameters {
            print_interval,
            driver_mode,
            num_inputs,
            connection_string,
            table_name,
            input_names,
            variant_name,
            primary_keys: vec![MetaCol::SimulationTime, MetaCol::Variant],
        };

        info!("Parameters: {:?}", parameters);

        Ok(parameters)
    }
}
