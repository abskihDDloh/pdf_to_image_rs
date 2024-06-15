use sysinfo::System;

/// Returns the maximum number of main workers based on the number of CPU cores.
pub fn get_main_workers_limit() -> usize {
    let core_count: usize = match num_cpus::get() {
        0 => 1,
        n => n,
    };
    let workers_limit: usize = (core_count - 1) / 4;
    if workers_limit == 0 {
        return 1;
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

/// Returns the maximum number of sub workers based on the number of CPU cores and boost percentage.
///
/// # Arguments
///
/// * `boost_percentage` - The boost percentage for sub workers. If the CPU usage is below this percentage, the number of sub workers will be doubled.
///
/// # Returns
///
/// The maximum number of sub workers.
pub fn get_sub_workers_limit(boost_percentage: f32) -> usize {
    let core_count: usize = match num_cpus::get() {
        0 => 1,
        n => n,
    };
    let main_workers = get_main_workers_limit();
    let sub_workers_limit: usize = if main_workers > 0 {
        (core_count - main_workers) / main_workers
    } else {
        0
    };
    let sub_workers_limit: usize = match sub_workers_limit {
        0 => 1,
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
