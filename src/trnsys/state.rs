use crate::storage::StoreProvider;
use crate::trnsys::iteration_mode::IterationMode;
use crate::trnsys::param::TrnSysValue;
use crate::trnsys::{
    get_current_unit, get_input_value, get_label, get_output_value, get_parameter_value,
};

pub(crate) struct TrnSysState<D = (), S = ()> {
    pub(crate) trnsys_standard_version: i32,
    pub(crate) num_params: i32,
    pub(crate) params: Vec<TrnSysValue>,
    pub(crate) num_labels: i32,
    pub(crate) labels: Vec<String>,
    pub(crate) num_inputs: i32,
    pub(crate) inputs: Vec<TrnSysValue>,
    pub(crate) num_derivatives: i32,
    pub(crate) num_outputs: i32,
    pub(crate) default_output_values: Vec<TrnSysValue>,
    pub(crate) outputs: Vec<TrnSysValue>,
    pub(crate) iteration_mode: IterationMode,
    /// The number of stored variables （static, dynamic）
    // pub(crate) num_stored_variables: (i32, i32),
    // pub(crate) static_store: Vec<TrnSysValue>,
    // pub(crate) variable_store: Vec<TrnSysValue>,
    pub(crate) dynamic_store: Option<Box<dyn StoreProvider<D>>>,
    pub(crate) static_store: Option<Box<dyn StoreProvider<S>>>,
}

impl TrnSysState {
    /// set up parameters for the TRNSYS type
    pub fn new() -> Self {
        TrnSysState {
            trnsys_standard_version: 18,
            iteration_mode: IterationMode::default(),

            num_params: 0,
            params: vec![],

            num_labels: 0,
            labels: vec![],

            num_inputs: 0,
            inputs: vec![],

            num_derivatives: 0,

            num_outputs: 0,
            outputs: vec![],
            default_output_values: vec![],

            // num_stored_variables: (0, 0),
            // TODO: Use storage if you need to read states from last iteration/time step
            static_store: None,
            dynamic_store: None,
        }
    }

    pub fn read_parameter_values(&mut self) {
        self.params = (1..self.num_params + 1)
            .map(|i| TrnSysValue {
                value: get_parameter_value(i),
            })
            .collect();

        self.labels = (1..self.num_labels + 1)
            .map(|i| get_label(get_current_unit(), i))
            .collect();
    }

    pub fn read_input_values(&mut self) {
        self.inputs = (1..self.num_inputs + 1)
            .map(|i| TrnSysValue {
                value: get_input_value(i),
            })
            .collect();

        self.outputs = (1..self.num_outputs + 1)
            .map(|i| TrnSysValue {
                value: get_output_value(i),
            })
            .collect();
    }
}
