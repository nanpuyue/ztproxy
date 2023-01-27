use std::env;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::thread;

use libzt::tcp::TcpListener;

// (Optional) Notify application of ZeroTier events, some with context
fn user_event_handler(event_code: i16) {
    println!("user_event {}", event_code);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    if args.len() != 5 {
        println!("Incorrect number of arguments.");
        println!("  Usage: ztproxy <storage_path> <net_id> <listen_port> <target_addr>");
        return Ok(());
    }

    let storage_path = &args[1];
    let net_id = u64::from_str_radix(&args[2], 16).unwrap();

    println!("path   = {}", storage_path);
    println!("net_id = {:x}", net_id);

    // SET UP ZEROTIER

    let node = libzt::node::ZeroTierNode {};
    // (Optional) initialization
    node.init_set_port(0);
    node.init_set_event_handler(user_event_handler);
    node.init_from_storage(storage_path);
    // Start the node
    node.start();
    println!("Waiting for node to come online...");
    while !node.is_online() {
        node.delay(50);
    }
    println!("Node ID = {:#06x}", node.id());
    println!("Joining network");
    node.net_join(net_id);
    println!("Waiting for network to assign addresses...");
    while !node.net_transport_is_ready(net_id) {
        node.delay(50);
    }
    let addr = node.addr_get(net_id).unwrap();
    println!("Assigned addr = {}", addr);

    let local_port = args[3].parse().expect("Unable to parse listen port");
    let target_addr: SocketAddr = args[4].parse().expect("");

    let listen_addr = std::net::SocketAddr::from((addr, local_port));
    let listener = TcpListener::bind(&listen_addr).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection: {}", stream.peer_addr().unwrap());
                let stream = Arc::new(stream);
                let upstream =
                    Arc::new(TcpStream::connect(target_addr).expect("Connect to target addr err"));

                let stream2 = stream.clone();
                let upstream2 = upstream.clone();
                thread::spawn(move || {
                    let _ = io::copy(&mut stream.as_ref(), &mut upstream.as_ref());
                });
                thread::spawn(move || {
                    let _ = io::copy(&mut upstream2.as_ref(), &mut stream2.as_ref());
                });
            }
            Err(e) => {
                println!("Error: {}", e);
                // connection failed
            }
        }
    }

    node.stop();
    Ok(())
}
