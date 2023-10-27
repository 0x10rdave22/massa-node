// Copyright (c) 2022 MASSA LABS <info@massa.net>

#[cfg(any(
    test,
    feature = "gas_calibration",
    feature = "benchmarking",
    feature = "test-exports",
    test
))]
mod mock;

#[cfg(all(not(feature = "gas_calibration"), not(feature = "benchmarking")))]
mod scenarios_mandatories;

#[cfg(all(not(feature = "gas_calibration"), not(feature = "benchmarking")))]
mod tests_active_history;

mod interface;

#[cfg(any(
    feature = "gas_calibration",
    feature = "benchmarking",
    feature = "test-exports",
    test
))]
pub use mock::get_sample_state;
