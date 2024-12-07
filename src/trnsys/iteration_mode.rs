

/// The IterationMode enum is used to specify the iteration mode of a Type.
/// It determines when and how often the Type is called during the simulation. \
/// Usually, the static-mode types are called first, followed by the dynamic-mode types, \
/// and finally the after-convergence types. \
/// After that, Integrators and Printers are called. \
/// Finally, the after-convergence-and-printers types are called.
#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum IterationMode {
    /// The Type's outputs only depend upon its input values and not explicitly upon time.
    StaticMode = 0,
    /// The Type's outputs depend upon the passage of time and the Type must therefore be called
    /// at least once every time step even if the values of inputs do not change
    DynamicMode = 1,
    /// The Type should be called after all other components have converged and before the
    /// integrators and printers.
    AfterConvergence = 3,
    /// Integration Mode
    IntegrationMode = 4,
    /// Printer Mode
    PrinterMode = 5,
    /// The Type should be called after all other components have converged and after the
    /// integrators and printers.
    AfterConvergenceAndPrinters = 2,
}

impl Default for IterationMode {
    fn default() -> Self {
        IterationMode::DynamicMode
    }
}

impl From<IterationMode> for i32 {
    fn from(mode: IterationMode) -> Self {
        mode as i32
    }
}