use regex::Regex;
use std::thread::ThreadId;

/// Retrieves the numeric ID of a thread.
///
/// # Arguments
///
/// * `id` - The `ThreadId` of the thread.
///
/// # Returns
///
/// An `Option` containing the numeric ID of the thread, or `None` if the ID cannot be parsed.
///
/// # Workaround
///
/// This function is a workaround for the issue.
/// For more details, see the following issue: [GitHub Issue #67939](https://github.com/rust-lang/rust/issues/67939)
fn _get_thread_id_number(id: &ThreadId) -> Option<u64> {
    let thread_id_str = format!("{:?}", id);

    let re = Regex::new(r"\d+").unwrap();
    re.captures(&thread_id_str)
        .and_then(|cap| cap.get(0).map(|m| m.as_str().parse().ok()))
        .flatten()
}

/// Retrieves the numeric ID of a thread.
///
/// # Arguments
///
/// * `id` - The `ThreadId` of the thread.
///
/// # Returns
///
/// The numeric ID of the thread. If the ID cannot be parsed, it returns 0.
pub fn get_thread_id_number(id: &ThreadId) -> u64 {
    let res: Option<u64> = _get_thread_id_number(id);
    match res {
        Some(num) => num,
        None => 0,
    }
}
