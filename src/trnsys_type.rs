use crate::database::buffer::DataBuffer;
use crate::database::column::MetaCol;
use crate::database::datatype::{ColDataType, ColDef};
use crate::database::ms_access::MsAccessProvider;
use crate::database::ms_excel::MsExcelProvider;
use crate::database::odbc::{FileDbProvider, OdbcProvider, OdbcProviderImpl};
use crate::database::sqlite::SqliteProvider;
use crate::parameter::{DriverMode, Parameters};
use crate::trnsys::error::TrnSysError;
use crate::trnsys::param::TrnSysValue;
use crate::trnsys::*;
use odbc_api::Environment;
use std::sync::LazyLock;
use tracing::info;
use tracing_subscriber::fmt::format;

static ENVIRONMENT: LazyLock<Environment> = LazyLock::new(|| Environment::new().unwrap());

pub(crate) struct TrnSysType {
    parameters: Option<Parameters>,
    db_provider: Option<Box<dyn OdbcProvider<'static>>>,
    last_recorded_no: u32,
    buffer: Vec<DataBuffer>,
}
impl TrnSysType {
    /// set up parameters for the TRNSYS type
    pub fn new() -> Self {
        TrnSysType {
            parameters: None,
            db_provider: None,
            last_recorded_no: 0,
            buffer: vec![],
        }
    }

    /// The very first call of the simulation.
    /// At this time, **only the number of parameters, inputs, or outputs are available**.
    /// You can change the number of parameters, inputs, or outputs at this time.
    /// If inconsistent, error will be automatically raised.
    pub fn first_call_of_simulation(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // All the "Very First Call of the Simulation Manipulations"

        Ok(())
    }

    /// Validate the input parameters.
    /// If not valid, raise `InputError::BadInput` or `InputError::BadParameter` to stop the simulation.
    pub fn validate_parameters(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Validate the parameters
        self.parameters = Some(Parameters::try_from(&*state)?);

        Ok(())
    }
    /// This function is called at the beginning of each simulation.
    /// Do start calculations here and store the results in the static store
    pub fn simulation_starts(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        info!("Simulation Starts. Connecting to Database...");
        let params = self.parameters.as_ref().unwrap();
        let provider: Box<dyn OdbcProvider> = match params.driver_mode {
            DriverMode::ConnectionString => {
                let mut db_provider = OdbcProviderImpl::new();
                db_provider.setup_by_conn_str(
                    &ENVIRONMENT,
                    params.connection_string.as_str(),
                    None,
                )?;
                Box::new(db_provider)
            }
            DriverMode::MsAccessFile => {
                let mut db_provider = MsAccessProvider::new();
                db_provider.setup_by_path(&ENVIRONMENT, params.connection_string.as_str(), None)?;
                Box::new(db_provider)
            }
            DriverMode::MsExcelFile => {
                let mut db_provider = MsExcelProvider::new();
                db_provider.setup_by_path(&ENVIRONMENT, params.connection_string.as_str(), None)?;
                Box::new(db_provider)
            }
            DriverMode::SqliteFile => {
                let mut db_provider = SqliteProvider::new();
                db_provider.setup_by_path(&ENVIRONMENT, params.connection_string.as_str(), None)?;
                Box::new(db_provider)
            }
        };

        // Format the fields
        let input_names = params
            .input_names
            .iter()
            .map(|s| ColDef::new(s, ColDataType::Number { decimal: true }, true, false))
            .collect::<Vec<_>>();

        self.db_provider = Some(provider);

        let db = self.db_provider.as_mut().unwrap();

        db.ensure_table(&params.table_name, input_names, None)?;

        // Remove existing variant data
        db.remove_variant(&params.table_name, &params.variant_name)?;

        Ok(())
    }

    /// Whether the simulation ends correctly or ends in error, each Type is recalled by the TRNSYS
    /// kernel before the simulation shuts down.
    pub fn simulation_ends(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Do all of the Last Call Manipulations Here
        info!("Simulation Ends");
        self.write_buffer()?;
        self.db_provider = None;
        Ok(())
    }

    /// The TRNSYS kernel calls this function at each time step. \
    /// This function will be called one or more times at each time step. \
    /// This function should return the values of the outputs for the current time step. \
    /// TrnSys will take care of the convergence of the simulation.
    pub fn iterate(&mut self, state: &mut TrnSysState) -> Result<Vec<TrnSysValue>, TrnSysError> {
        Ok(vec![])
    }

    /// At the end of each time step, each Type in a simulation is recalled.
    /// If necessary, store the values of the outputs for the current time step
    /// in the dynamic storage
    pub fn end_of_timestep(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Perform Any "End of Timestep" Manipulations That May Be Required
        if self.get_new_record_no() == self.last_recorded_no {
            return Ok(());
        }

        // Insert data
        let row = state.inputs.iter().map(|v| v.value).collect::<Vec<f64>>();
        let mut buffer_row = DataBuffer::new(Some(row));

        // Insert meta columns
        let params = self.parameters.as_ref().unwrap();
        buffer_row.insert_meta_col(MetaCol::SimulationTime, get_simulation_time());
        buffer_row.insert_meta_col(MetaCol::Variant, params.variant_name.clone());
        self.buffer.push(buffer_row);

        if self.is_time_to_write_buffer() {
            self.write_buffer()?;
            self.buffer.clear();
        }
        Ok(())
    }

    pub fn get_default_output_values(
        &self,
        state: &mut TrnSysState,
    ) -> Result<Vec<TrnSysValue>, TrnSysError> {
        // initialize output values
        let default_outputs = (1..(state.num_outputs + 1))
            .map(|i| TrnSysValue { value: 0. })
            .collect();
        Ok(default_outputs)
    }

    fn get_new_record_no(&self) -> u32 {
        let interval = self.parameters.as_ref().unwrap().print_interval;
        let simulation_time = get_simulation_time();

        (simulation_time / interval) as u32
    }

    fn is_time_to_write_buffer(&self) -> bool {
        self.buffer.len() >= 24 * 10 || get_simulation_stop_time() == get_simulation_time()
    }

    fn write_buffer(&mut self) -> Result<(), TrnSysError> {
        if self.buffer.len() == 0 {
            return Ok(());
        }
        let db_provider = self.db_provider.as_mut().unwrap();
        let params = self.parameters.as_ref().unwrap();

        for row in self.buffer.drain(..) {
            let insertable = row.into_insertable(&self.parameters.as_ref().unwrap().input_names);
            db_provider.insert_data(&params.table_name, insertable)?;
        }
        self.buffer.clear();
        Ok(())
    }
}
