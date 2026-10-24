//! TCP listening port discovery for a process tree.

use std::process::Command;

/// Return listening TCP ports owned by a process and its descendants.
pub fn listening(pid: u32) -> Vec<u16> {
    let mut process_ids = vec![pid];
    let mut index = 0;
    while index < process_ids.len() {
        if let Ok(output) = Command::new("pgrep")
            .args(["-P", &process_ids[index].to_string()])
            .output()
        {
            process_ids.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|process_id| process_id.parse::<u32>().ok()),
            );
        }
        index += 1;
    }
    let Ok(output) = Command::new("lsof")
        .args([
            "-nP",
            "-a",
            "-iTCP",
            "-sTCP:LISTEN",
            "-p",
            &process_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        ])
        .output()
    else {
        return vec![];
    };
    let mut ports = String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(8))
        .filter_map(|address| address.rsplit(':').next()?.parse::<u16>().ok())
        .collect::<Vec<_>>();
    ports.sort_unstable();
    ports.dedup();
    ports
}
