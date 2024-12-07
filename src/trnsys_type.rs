use crate::storage::StoreProvider;
use crate::trnsys::error::{InputError, TrnSysError};
use crate::trnsys::iteration_mode::IterationMode;
use crate::trnsys::param::TrnSysValue;
use crate::trnsys::*;
use tracing::{info, warn};

pub(crate) struct TrnSysType {}
impl TrnSysType {
    /// set up parameters for the TRNSYS type
    pub fn new() -> Self {
        TrnSysType {}
    }

    /// The very first call of the simulation.
    /// At this time, **only the number of parameters, inputs, or outputs are available**.
    /// You can change the number of parameters, inputs, or outputs at this time.
    /// If inconsistent, error will be automatically raised.
    pub fn first_call_of_simulation(&self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // All the "Very First Call of the Simulation Manipulations"
        // TODO: Set the number of parameters, inputs, outputs, and derivatives
        state.num_inputs = 1;
        state.num_params = 1;
        state.num_derivatives = 0;
        state.num_outputs = 1;
        Ok(())
    }

    /// Validate the input parameters.
    /// If not valid, raise `InputError::BadInput` or `InputError::BadParameter` to stop the simulation.
    pub fn validate_parameters(&self, state: &mut TrnSysState) -> Result<(), InputError> {
        // Validate the parameters
        let param0: i32 = state
            .params
            .get(0)
            .ok_or(InputError::BadParameter {
                index: 0,
                message: "Parameter 1 is missing".to_string(),
            })?
            .try_into()
            .map_err(|e| InputError::BadParameter {
                index: 0,
                message: format!("{:?}", e),
            })?;

        info!("Parameter 0: {}", param0);
        if param0 <= 0 {
            warn!("Parameter 0 is less than or equal to 0");
            return Err(InputError::BadParameter {
                index: 0,
                message: "Parameter 1 must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
    /// This function is called at the beginning of each simulation.
    /// Do start calculations here and store the results in the static store
    pub fn simulation_starts(&self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        info!("Simulation Starts");
        Ok(())
    }

    /// Whether the simulation ends correctly or ends in error, each Type is recalled by the TRNSYS
    /// kernel before the simulation shuts down.
    pub fn simulation_ends(&self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Do All of the Last Call Manipulations Here
        info!("Simulation Ends");
        Ok(())
    }

    /// The TRNSYS kernel calls this function at each time step. \
    /// This function will be called one or more times at each time step. \
    /// This function should return the values of the outputs for the current time step. \
    /// TrnSys will take care of the convergence of the simulation.
    pub fn iterate(&self, state: &mut TrnSysState) -> Result<Vec<TrnSysValue>, TrnSysError> {
        let time = get_simulation_time();
        let timestep = get_simulation_time_step();
        let current_unit = get_current_unit();
        let current_type = get_current_type();
        let first: f64 = state.inputs.get(0).unwrap().into();
        let param0: i32 = state.params.get(0).unwrap().try_into()?;

        Ok(vec![(first * param0 as f64).into()])
    }

    /// At the end of each time step, each Type in a simulation is recalled.
    /// If necessary, store the values of the outputs for the current time step
    /// in the dynamic storage
    pub fn end_of_timestep(&self, state: &mut TrnSysState) -> Result<(), TrnSysError> {
        // Perform Any "End of Timestep" Manipulations That May Be Required
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
}
