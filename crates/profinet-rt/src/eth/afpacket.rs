use std::mem;
use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Duration;

use nix::sys::socket::{
    recv, send, socket, AddressFamily, MsgFlags, SockFlag, SockProtocol, SockType,
};

use super::transport::{EthTransport, TransportError};
use super::ETHERTYPE_PROFINET;

fn io_err<E: std::fmt::Display>(e: E) -> TransportError {
    TransportError::Io(e.to_string())
}

/// Raw AF_PACKET socket bound to a named interface, filtered on EtherType PROFINET at recv time.
pub struct AfPacketTransport {
    fd: OwnedFd,
}

impl AfPacketTransport {
    /// Open a raw AF_PACKET socket on `ifname`.
    ///
    /// Returns `Err(TransportError::Io)` if the interface does not exist or the
    /// process lacks `CAP_NET_RAW`.
    pub fn open(ifname: &str) -> Result<Self, TransportError> {
        // EthAll (ETH_P_ALL, already big-endian encoded by nix) captures every frame.
        let fd = socket(
            AddressFamily::Packet,
            SockType::Raw,
            SockFlag::empty(),
            SockProtocol::EthAll,
        )
        .map_err(io_err)?;

        // Resolve interface name -> index.  Returns ENODEV if unknown.
        let ifindex = nix::net::if_::if_nametoindex(ifname).map_err(io_err)?;

        // nix 0.27 LinkAddr has no public constructor, so we build sockaddr_ll directly.
        let mut sll: libc::sockaddr_ll = unsafe { mem::zeroed() };
        sll.sll_family = libc::AF_PACKET as u16;
        // ETH_P_ALL in network byte order (same value nix stores in SockProtocol::EthAll).
        sll.sll_protocol = (libc::ETH_P_ALL as u16).to_be();
        sll.sll_ifindex = ifindex as libc::c_int;

        let ret = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                &sll as *const libc::sockaddr_ll as *const libc::sockaddr,
                mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            return Err(io_err(std::io::Error::last_os_error()));
        }

        Ok(Self { fd })
    }
}

impl EthTransport for AfPacketTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        send(self.fd.as_raw_fd(), frame, MsgFlags::empty()).map_err(io_err)?;
        Ok(())
    }

    /// Returns `Ok(Some(frame))` only for frames whose EtherType is PROFINET (0x8892).
    /// Returns `Ok(None)` for any other frame.
    ///
    /// Timeout support (via `SO_RCVTIMEO` / `poll`) is deferred to Plan 4 when the RT
    /// loop requires it; `_timeout` is accepted but ignored for now.
    fn recv(&self, _timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError> {
        let mut buf = vec![0u8; 1522];
        let n = recv(self.fd.as_raw_fd(), &mut buf, MsgFlags::empty()).map_err(io_err)?;
        buf.truncate(n);
        // Filter: only pass PROFINET frames (EtherType at byte offset 12, no VLAN awareness yet).
        if n >= 14 && u16::from_be_bytes([buf[12], buf[13]]) == ETHERTYPE_PROFINET {
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_unknown_interface_errors() {
        let r = AfPacketTransport::open("nonexistent-iface-xyz");
        assert!(r.is_err());
    }

    #[test]
    #[ignore = "necessite CAP_NET_RAW + une interface reelle ; lancer: cargo test -- --ignored"]
    fn open_loopback_succeeds() {
        // Adapter le nom d'interface a la machine de test (ex. "lo", "eth0").
        let t = AfPacketTransport::open("lo").expect("open lo");
        let _ = t.recv(Some(Duration::from_millis(10)));
    }
}
