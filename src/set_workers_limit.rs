use num_cpus;
use sysinfo::System;

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

/// Returns the CPU usage as a percentage.
fn get_cpu_usage() -> f32 {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_usage = sys.global_cpu_info().cpu_usage();
    cpu_usage
}

/// Returns the sub workers limit based on the boost percentage.
/// The sub workers limit is calculated as the difference between the total number of CPU cores and the main workers limit, divided by the main workers limit.
/// If the main workers limit is 0 or less, the sub workers limit is set to 0.
/// If the boost percentage is less than 0.0 or greater than 60.0, the sub workers limit remains unchanged.
/// If the CPU usage is less than the boost percentage, the sub workers limit is doubled.
pub fn get_sub_workers_limit(boost_percentage: f32) -> usize {
    let core_count: usize = match num_cpus::get() {
        0 => 1,
        n => n as usize,
    };
    let main_workers = get_main_workers_limit();
    let sub_workers_limit: usize = if main_workers > 0 {
        (core_count - main_workers) / main_workers
    } else {
        0
    };
    let sub_workers_limit: usize = match sub_workers_limit {
        n if n <= 0 => 1,
        n => n,
    };
    if boost_percentage < 0.0 {
        return sub_workers_limit;
    }
    if boost_percentage > 60.0 {
        return sub_workers_limit;
    }
    if get_cpu_usage() < boost_percentage {
        sub_workers_limit * 2
    } else {
        sub_workers_limit
    }
}
