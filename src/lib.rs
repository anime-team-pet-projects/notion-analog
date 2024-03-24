use std::net::IpAddr;

mod infrastructure;
mod domain;
mod application;

pub fn run(ip_addr: IpAddr, port: u16) {
	println!("{}", format!("Application works on: {}:{}", {ip_addr}, {port}));
}
