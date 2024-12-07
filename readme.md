
# trnsys_rust_template

trnsys_rust_template provides you a modern, efficient and friendly way to create TrnSys types in Rust.
It is based on the TrnSys C++ Template, which can be found in your TrnSys installation folder.
With its error handling, logging and other features, it's much easier to write, test and debug your own TrnSys type.
## Usage

1. Clone this repository
2. Paste the `TRNDll64.lib` under the `lib` folder
3. Change the project name in `Cargo.toml` if needed
4. Set your type number in `Cargo.toml` in section `[package.metadata.trnsys]`.
5. Write your calculation logic in the `src/trnsys_type.rs` file, where you can use all the functions in different
   simulation time as documented.

## Functionality

### Logging

Logging is enabled by default using `tracing` crate.
Logs are written to both the TrnSys lst/log file and a separate log file.

By default,

- only logs with level `INFO` or higher are written to the TrnSys log file.

- the separate log file is created in `%temp%` folder with the name `trnsys_{timpstamp}.log`. If the simulation finishes
  with no error, the file is deleted autimatically.

### Call TrnSys functions

Some functions are built-in in the `trnsys` module, which can be used to call TrnSys functions.

An example to get current timestep:

```rust
// use the trnsys module
use crate::trnsys::get_simulation_time_step;

fn some_function() {
   // get the current timestep
   let timestep = get_simulation_time_step();
}

```

### Error handling

Errors are handled using `thiserror` crate. You can add more error types in [src/trnsys/error.rs](src/trnsys/error.rs)
file.

If you would like to perform more actions when an error occurs, you can implement the `TrnSysErrorHandler` trait for
your error type and modify the handling process accordingly.

## Advanced

### Add more functions
You can add more functions to the `src/trnsys/ext_c.rs` file and add the corresponding rust-flavored function in the `src/trnsys/mod.rs` file.
