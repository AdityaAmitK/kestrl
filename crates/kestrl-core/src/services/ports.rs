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
    let mut ports = listening_ports(&process_ids);
    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Listening ports from `lsof`; empty when it is unavailable.
#[cfg(not(target_os = "linux"))]
fn listening_ports(process_ids: &[u32]) -> Vec<u16> {
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
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .skip(1)
        .filter_map(|line| line.split_whitespace().nth(8))
        .filter_map(|address| address.rsplit(':').next()?.parse::<u16>().ok())
        .collect()
}

/// Listening ports from `/proc`: socket inodes the processes hold, matched against TCP sockets
/// in the LISTEN state. Needs no extra tools, which minimal servers often lack.
#[cfg(target_os = "linux")]
fn listening_ports(process_ids: &[u32]) -> Vec<u16> {
    use std::collections::HashSet;
    use std::fs;

    const LISTEN: &str = "0A";
    let inodes: HashSet<String> = process_ids
        .iter()
        .filter_map(|process_id| fs::read_dir(format!("/proc/{process_id}/fd")).ok())
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter_map(|target| {
            target
                .to_str()?
                .strip_prefix("socket:[")?
                .strip_suffix(']')
                .map(str::to_owned)
        })
        .collect();
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .iter()
        .filter_map(|table| fs::read_to_string(table).ok())
        .flat_map(|text| {
            text.lines()
                .skip(1)
                .filter_map(|line| {
                    // sl local_address rem_address st tx:rx tr:when retrnsmt uid timeout inode
                    let fields: Vec<&str> = line.split_whitespace().collect();
                    if fields.get(3) != Some(&LISTEN) || !inodes.contains(*fields.get(9)?) {
                        return None;
                    }
                    u16::from_str_radix(fields[1].rsplit(':').next()?, 16).ok()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
