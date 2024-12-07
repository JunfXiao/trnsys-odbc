use crate::logging::init_tracing;
use crate::trnsys::error::{TrnSysError, TrnSysErrorHandler};
use crate::trnsys::logging::cleanup_tracing;
use crate::trnsys_type::TrnSysType;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, LazyLock, RwLock};
use tracing::{debug, error, info};
use trnsys::*;

mod storage;
mod trnsys;
mod trnsys_type;

include!(concat!(env!("OUT_DIR"), "/generated_entrance.rs"));

static TRNSYS_STATE_DICT: LazyLock<RwLock<HashMap<i32, Arc<RwLock<TrnSysState>>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static TRNSYS_TYPE_INSTANCE: LazyLock<Arc<TrnSysType>> = LazyLock::new(|| {
    // initialize the logging only once
    init_tracing(None);
    Arc::new(TrnSysType::new())
});

fn get_current_state() -> Arc<RwLock<TrnSysState>> {
    let mut dict = (&TRNSYS_STATE_DICT).write().unwrap();
    let unit = get_current_unit();

    dict.entry(unit)
        .or_insert(Arc::new(RwLock::new(TrnSysState::new())))
        .clone()
}

fn entrance() {
    let state_lock = get_current_state();
    let mut state = state_lock.write().unwrap();
    // create type instance
    match main(state.deref_mut()) {
        Ok(_) => {}
        Err(e) => {
            e.handle_in_trnsys(state.deref_mut());
            error!("{:?}", e);
        }
    }
}

fn main(mut state: &mut TrnSysState) -> Result<(), TrnSysError> {
    let type_instance = TRNSYS_TYPE_INSTANCE.clone();

    if is_version_signing_time() {
        set_type_version(state.trnsys_standard_version);
        return Ok(());
    } else if is_first_call_of_simulation() {
        // Tell the TRNSYS Engine How This Type Works
        state.num_inputs = get_number_of_inputs();
        state.num_params = get_number_of_parameters();
        state.num_outputs = get_number_of_outputs();
        state.num_derivatives = get_number_of_derivatives();

        type_instance.first_call_of_simulation(&mut state)?;

        debug!("Number of Inputs: {}", state.num_inputs);
        debug!("Number of Parameters: {}", state.num_params);
        debug!("Number of Outputs: {}", state.num_outputs);
        debug!("Number of Derivatives: {}", state.num_derivatives);

        set_number_of_parameters(state.num_params);
        set_number_of_inputs(state.num_inputs);
        set_number_of_derivatives(state.num_derivatives);
        set_number_of_outputs(state.num_outputs);
        set_iteration_mode(state.iteration_mode.into());
        return Ok(());
    }
    state.read_input_values();
    state.read_parameter_values();

    // read_storage(&mut state);

    if is_last_call_of_simulation() {
        type_instance.simulation_ends(&mut state)?;
        cleanup_tracing();
        return Ok(());
    }

    if is_end_of_timestep() {
        type_instance.end_of_timestep(&mut state)?;
        return Ok(());
    }

    if is_start_time() {
        // validate parameters
        type_instance.validate_parameters(&mut state)?;
        // initialize outputs
        type_instance
            .get_default_output_values(&mut state)?
            .iter()
            .enumerate()
            .for_each(|(i, val)| {
                // attention: TRNSYS/Fortran is 1-indexed
                set_output_value(i as i32, val.value);
            });

        type_instance.simulation_starts(&mut state)?;
        return Ok(());
    }

    if is_reread_parameters() {
        state.read_parameter_values();
        state.read_input_values();
        // read_storage(&mut state);
    }
    // Perform All the Calculations Here
    let simulation_outputs = type_instance.iterate(&mut state)?;
    // set output
    simulation_outputs.iter().enumerate().for_each(|(i, val)| {
        // attention: TRNSYS/Fortran is 1-indexed
        set_output_value(i as i32, val.value);
    });

    Ok(())
}
