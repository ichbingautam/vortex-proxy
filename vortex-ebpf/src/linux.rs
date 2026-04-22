//! XDP eBPF loader using `aya` for Linux targets.

use super::XdpRateLimiter;
use aya::maps::HashMap;
use aya::programs::{Xdp, XdpFlags};
use aya::Bpf;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

/// Loads the pre-compiled XDP bytecodes into the Linux kernel
/// and attaches it to the specified network interface.
pub struct LinuxXdpLimiter {
    bpf: Arc<Mutex<Bpf>>,
    interface: String,
}

impl LinuxXdpLimiter {
    /// Initialize the XDP program. `bpf_code` must be an ELF binary compiled for the `bpf` target.
    pub fn new(
        interface: &str,
        bpf_code: &[u8],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut bpf = Bpf::load(bpf_code)?;

        // Attach to the network interface
        let program: &mut Xdp = bpf.program_mut("xdp_drop_ips").unwrap().try_into()?;
        program.load()?;
        program.attach(interface, XdpFlags::default())?;

        Ok(Self {
            bpf: Arc::new(Mutex::new(bpf)),
            interface: interface.to_string(),
        })
    }
}

impl XdpRateLimiter for LinuxXdpLimiter {
    fn block_ip(&self, ip: IpAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let IpAddr::V4(ipv4) = ip {
            let mut bpf = self.bpf.lock().unwrap();
            let mut matched_ips: HashMap<_, u32, u32> =
                HashMap::try_from(bpf.map_mut("BLOCKED_IPS").unwrap())?;

            // Convert to network byte order
            let ip_bytes = u32::from_be_bytes(ipv4.octets());
            matched_ips.insert(ip_bytes, 1, 0)?;

            tracing::info!("XDP: Blocked V4 {}", ip);
        } else {
            tracing::warn!("XDP blocker currently only supports IPv4");
        }
        Ok(())
    }

    fn unblock_ip(&self, ip: IpAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let IpAddr::V4(ipv4) = ip {
            let mut bpf = self.bpf.lock().unwrap();
            let mut matched_ips: HashMap<_, u32, u32> =
                HashMap::try_from(bpf.map_mut("BLOCKED_IPS").unwrap())?;

            let ip_bytes = u32::from_be_bytes(ipv4.octets());
            matched_ips.remove(&ip_bytes)?;

            tracing::info!("XDP: Unblocked V4 {}", ip);
        }
        Ok(())
    }
}
