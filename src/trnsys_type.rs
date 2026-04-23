use crate::database::buffer::DataBuffer;
use crate::database::column::MetaCol;
use crate::database::datatype::{ColDataType, ColDef};
use crate::database::ms_access::MsAccessProvider;
use crate::database::ms_excel::MsExcelProvider;
use crate::database::odbc::{FileDbProvider, OdbcProvider, OdbcProviderImpl};
use crate::database::postgres::PostgreSQLProvider;
use crate::database::sqlite::SqliteProvider;
use crate::parameter::{DriverMode, Parameters};
use crate::trnsys::error::TrnSysError;
use crate::trnsys::param::TrnSysValue;
use crate::trnsys::*;
use odbc_api::Environment;
use std::sync::mpsc;
use std::sync::LazyLock;
use std::thread;
use tracing::{debug, info};

static ENVIRONMENT: LazyLock<Environment> = LazyLock::new(|| Environment::new().unwrap());

/// Payload sent from simulation thread to writer thread.
struct WriteBatch {
    sim_times: Vec<f64>,
    input_rows: Vec<Vec<f64>>,
}

/// Bounded channel capacity — simulation can produce up to 3 batches ahead before blocking.
const CHANNEL_CAPACITY: usize = 3;

/// Owns the DB provider on a background thread; accepts WriteBatch via bounded channel.
struct BackgroundWriter {
    sender: Option<mpsc::SyncSender<WriteBatch>>,
    handle: Option<thread::JoinHandle<Result<(), TrnSysError>>>,
}

impl BackgroundWriter {
    fn new(
        db_provider: Box<dyn OdbcProvider<'static>>,
        table_name: String,
        variant_id_col: String,
        simtime_col: String,
        input_col_names: Vec<String>,
        variant_id: i32,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel::<WriteBatch>(CHANNEL_CAPACITY);
        let handle = thread::spawn(move || -> Result<(), TrnSysError> {
            while let Ok(batch) = rx.recv() {
                let t = std::time::Instant::now();
                db_provider.columnar_batch_insert(
                    &table_name,
                    &variant_id_col,
                    &simtime_col,
                    &input_col_names,
                    variant_id,
                    &batch.sim_times,
                    &batch.input_rows,
                )?;
                debug!(
                    "[bg-writer] wrote {} rows in {:?}",
                    batch.sim_times.len(),
                    t.elapsed()
                );
            }
            // All batches written — mark the variant as complete
            db_provider.mark_variant_complete(variant_id)?;
            Ok(())
        });
        BackgroundWriter {
            sender: Some(tx),
            handle: Some(handle),
        }
    }

    /// Send a batch to the writer. Blocks only if CHANNEL_CAPACITY batches are queued.
    fn send(&self, batch: WriteBatch) -> Result<(), TrnSysError> {
        if let Some(ref sender) = self.sender {
            sender
                .send(batch)
                .map_err(|_| TrnSysError::GeneralError("Background writer thread died".into()))?;
        }
        Ok(())
    }

    /// Close channel, wait for writer to finish, propagate any errors.
    fn flush(&mut self) -> Result<(), TrnSysError> {
        // Drop sender to signal the writer thread to exit
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(result) => result?,
                Err(_) => {
                    return Err(TrnSysError::GeneralError(
                        "Background writer thread panicked".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct TrnSysType {
    parameters: Option<Parameters>,
    db_provider: Option<Box<dyn OdbcProvider<'static>>>,
    writer: Option<BackgroundWriter>,
    last_recorded_no: u32,
    buffer: Vec<DataBuffer>,
}

const ROW_BUFFER_SIZE: usize = 24 * 30;

impl TrnSysType {
    /// set up parameters for the TRNSYS type
    pub fn new() -> Self {
        TrnSysType {
            parameters: None,
            db_provider: None,
            writer: None,
            last_recorded_no: 0,
            buffer: Vec::with_capacity(ROW_BUFFER_SIZE),
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
            DriverMode::Postgres => {
                let mut db_provider = PostgreSQLProvider::new();
                db_provider.setup_by_conn_str(
                    &ENVIRONMENT,
                    params.connection_string.as_str(),
                    None,
                )?;
                Box::new(db_provider)
            }
        };

        // Format the fields
        let input_names = params
            .input_names
            .iter()
            .map(|s| ColDef::new(s, ColDataType::Number { decimal: true }, false, false))
            .collect::<Vec<_>>();

        self.db_provider = Some(provider);

        let db = self.db_provider.as_ref().unwrap();

        // Ensure the shared variants lookup table, then resolve this run's variant_id.
        db.ensure_variants_table()?;
        let variant_id = db.ensure_variant(&params.variant_name)?;
        info!(
            "Resolved variant '{}' -> variant_id={}",
            params.variant_name, variant_id
        );

        db.ensure_table(&params.table_name, input_names, None)?;

        // Clear any existing rows for this variant
        db.remove_variant_data(&params.table_name, variant_id)?;

        // Move the provider into a background writer thread
        let writer = BackgroundWriter::new(
            self.db_provider.take().unwrap(),
            params.table_name.clone(),
            MetaCol::VariantId.as_str().to_string(),
            MetaCol::SimulationTime.as_str().to_string(),
            params.input_names.clone(),
            variant_id,
        );
        self.writer = Some(writer);

        Ok(())
    }

    /// Whether the simulation ends correctly or ends in error, each Type is recalled by the TRNSYS
    /// kernel before the simulation shuts down.
    pub fn simulation_ends(&mut self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Do all of the Last Call Manipulations Here
        info!("Simulation Ends");
        self.write_buffer()?;
        if let Some(mut writer) = self.writer.take() {
            writer.flush()?;
        }
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
        if self.get_new_record_no(state.simulation_time) == self.last_recorded_no {
            return Ok(());
        } else {
            self.last_recorded_no = self.get_new_record_no(state.simulation_time);
        }

        // Insert data
        let row = state.inputs.iter().map(|v| v.value).collect::<Vec<f64>>();
        let mut buffer_row = DataBuffer::new(Some(row));
        buffer_row.sim_time = state.simulation_time;
        self.buffer.push(buffer_row);

        if self.is_time_to_write_buffer(state.simulation_time) {
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

    fn get_new_record_no(&self, simulation_time: f64) -> u32 {
        let interval = self.parameters.as_ref().unwrap().print_interval;

        (simulation_time / interval) as u32
    }

    fn is_time_to_write_buffer(&self, simulation_time: f64) -> bool {
        self.buffer.len() >= ROW_BUFFER_SIZE - 10 || get_simulation_stop_time() == simulation_time
    }

    fn write_buffer(&mut self) -> Result<(), TrnSysError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let row_count = self.buffer.len();

        // Extract data in columnar form
        let sim_times: Vec<f64> = self.buffer.iter().map(|r| r.sim_time).collect();
        let input_rows: Vec<Vec<f64>> = self.buffer.drain(..).map(|r| r.input_data).collect();

        if let Some(ref writer) = self.writer {
            writer.send(WriteBatch {
                sim_times,
                input_rows,
            })?;
        }

        info!("Queued {} rows for background write", row_count);
        Ok(())
    }
}
