use std::net::UdpSocket;
use std::time::Duration;
use std::io;
use tokio::signal;
use tokio_cron_scheduler::{Job, JobScheduler};
use sysinfo::System;

const BIND_ADDR: &str = "0.0.0.0:0";
const SERVER_ADDR: &str = "127.0.0.1:6076";
const BUFFER_SIZE: usize = 1024;
const READ_TIMEOUT_SECS: u64 = 5;
const STATUS_COMMAND: &str = "status";
const PROCESS_NAME: &str = "M17Gateway";

// Create a UDP socket
fn create_socket() -> io::Result<UdpSocket> {
    let socket = UdpSocket::bind(BIND_ADDR).expect("[xorctl] Could not bind socket");
    socket.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS)))?;
    Ok(socket)
}

// Send a command via UDP and receive a response
fn send_udp_command(command: &str) -> io::Result<String> {
    let socket = create_socket()?;
    socket.send_to(command.as_bytes(), SERVER_ADDR).expect("[xorctl] Could not send command");
    let mut buffer = [0; BUFFER_SIZE];
    let (bytes_received, _) = socket.recv_from(&mut buffer).expect("[xorctl] Didn't receive data");
    Ok(String::from_utf8_lossy(&buffer[..bytes_received]).to_string())
}

// Check for disconnection and attempt to reconnect
fn check_and_reconnect() {
    let response: String = send_udp_command(STATUS_COMMAND).unwrap();
    if response.contains("disc") {
        println!("[xorctl] Lost connection, restarting {}...", PROCESS_NAME);
        restart_process(PROCESS_NAME);
    } else {
        println!("[xorctl] Connection ok...");
    }
}

// Restart the specified process
fn restart_process(process_name: &str) {
    let mut system = System::new_all();
    system.refresh_all();

    for (pid, process) in system.processes() {
        if process.name() == process_name {
            println!("[xorctl] Found {} with PID {}", process_name, pid);
            process.kill();
        }
    }
}

#[tokio::main]
async fn main() {
    let mut scheduler = JobScheduler::new().await.unwrap();
    scheduler.add(
        Job::new("1/30 * * * * *", |_uuid, _l| {
            check_and_reconnect();
        }).unwrap()
    ).await.unwrap();

    scheduler.start().await.unwrap();
    signal::ctrl_c().await.unwrap();
}