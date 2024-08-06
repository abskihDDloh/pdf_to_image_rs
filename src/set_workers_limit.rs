use sysinfo::{CpuRefreshKind, RefreshKind, System};

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
    let mut s =
        System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    s.refresh_cpu_usage();
    s.global_cpu_usage()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_main_workers_limit() {
        let main_workers_limit = get_main_workers_limit();
        //workers_limitが1以上の数値であることを確認する。
        assert!(main_workers_limit > 0);
    }

    #[test]
    fn test_get_sub_workers_limit() {
        let sub_worker_limit_pm0p1 = get_sub_workers_limit(-0.1);
        assert!(sub_worker_limit_pm0p1 > 0);

        let sub_worker_limit_p0 = get_sub_workers_limit(0.0);
        assert!(sub_worker_limit_p0 > 0);

        let sub_worker_limit_p60 = get_sub_workers_limit(60.0);
        assert!(sub_worker_limit_p60 > 0);

        let sub_worker_limit_p60p1 = get_sub_workers_limit(60.1);
        assert!(sub_worker_limit_p60p1 > 0);

        //boost_percentageが-0.1の場合と60.1の場合はsub_workers_limitが変わらないことを確認する。
        assert_eq!(sub_worker_limit_pm0p1, sub_worker_limit_p60p1);

        //boost_percentageが0.0の場合よりも60.0の場合はsub_workers_limitが小さいことを確認する。
        assert!(sub_worker_limit_p0 <= sub_worker_limit_p60);

        //boost_percentageが60.0の場合よりも60.1の場合はsub_workers_limitが小さいことを確認する。
        assert!(sub_worker_limit_p60 >= sub_worker_limit_pm0p1);
    }
}
