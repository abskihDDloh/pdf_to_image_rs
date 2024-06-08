use num_cpus;

/// Returns the main workers limit based on the number of CPU cores.
/// The main workers limit is calculated as the number of CPU cores minus 1, divided by 4 and rounded down to the nearest integer.
/// If the calculated limit is 0 or less, it is set to 1.
pub fn get_main_workers_limit() -> usize {
    let core_count: usize = match num_cpus::get() {
        0 => 1,
        n => n as usize,
    };
    let workers_limit: usize = match (core_count - 1) / 4 {
        n if n <= 0 => 1,
        n => n,
    };
    workers_limit
}

/// Returns the sub workers limit based on the number of CPU cores and the main workers limit.
/// The sub workers limit is calculated as the result of ((number of CPU cores - main workers limit) / main workers limit),
/// rounded down to the nearest integer.
/// If the calculated limit is 0 or less, it is set to 1.
pub fn get_sub_workers_limit() -> usize {
    let core_count: usize = match num_cpus::get() {
        0 => 1,
        n => n as usize,
    };
    let main_workers = get_main_workers_limit();
    let _sub_workers_limit: usize = if main_workers > 0 {
        (core_count - main_workers) / main_workers
    } else {
        0
    };
    let sub_workers_limit: usize = match _sub_workers_limit {
        n if n <= 0 => 1,
        n => n,
    };
    sub_workers_limit
}