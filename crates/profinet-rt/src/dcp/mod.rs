//! PROFINET DCP (Discovery & Configuration Protocol) — device side.

pub mod block;
pub mod frame;
pub mod identify;

use thiserror::Error;

/// Errors from parsing/serializing DCP frames.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DcpError {
    #[error("buffer too short: need {need}, have {have}")]
    TooShort { need: usize, have: usize },
    #[error("unknown DCP service id {0}")]
    BadServiceId(u8),
    #[error("unknown DCP service type {0}")]
    BadServiceType(u8),
    #[error("unknown DCP frame id {0:#06x}")]
    BadFrameId(u16),
    #[error("malformed DCP frame: {0}")]
    Malformed(&'static str),
}

use crate::dcp::frame::{DcpHeader, FrameId};
use crate::dcp::identify::{build_identify_response, parse_identify_request, DeviceProperties};
use crate::eth::{EthHeader, MacAddr, ETHERTYPE_PROFINET};

/// Identity + address this device answers DCP for.
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub mac: MacAddr,
    pub properties: DeviceProperties,
}

/// Handle one received Ethernet frame. Returns the response frame to send, or
/// `None` if the frame is not a DCP request this device should answer.
pub fn handle_dcp_frame(frame: &[u8], cfg: &DeviceConfig) -> Result<Option<Vec<u8>>, DcpError> {
    let (eth, payload_off) =
        EthHeader::parse(frame).map_err(|_| DcpError::Malformed("bad Ethernet header"))?;
    if eth.ethertype != ETHERTYPE_PROFINET {
        return Ok(None);
    }
    let payload = &frame[payload_off..];
    if payload.len() < 2 {
        return Err(DcpError::TooShort {
            need: payload_off + 2,
            have: frame.len(),
        });
    }
    let frame_id = u16::from_be_bytes([payload[0], payload[1]]);
    match FrameId::from_u16(frame_id) {
        Some(FrameId::IdentifyRequest) => {
            let (header, blocks) = DcpHeader::parse(&payload[2..])?;
            let filter = parse_identify_request(blocks)?;
            let matches = match &filter.name_of_station {
                None => true,
                Some(name) => name == &cfg.properties.name_of_station,
            };
            if matches {
                Ok(Some(build_identify_response(
                    eth.src,
                    cfg.mac,
                    header.xid,
                    &cfg.properties,
                )))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::*;
    use crate::dcp::identify::DeviceProperties;
    use crate::eth::MacAddr;

    // Golden Identify REQUEST frame (56 bytes), docs/dcp-golden-frames.md.
    const REQ_FRAME: &[u8] = &[
        0x01, 0x0e, 0xcf, 0x00, 0x00, 0x00, 0xec, 0x1c, 0x5d, 0x61, 0xe7, 0x3f, 0x88, 0x92, 0xfe,
        0xfe, 0x05, 0x00, 0x03, 0x00, 0x01, 0x52, 0x00, 0x01, 0x00, 0x0c, 0x02, 0x02, 0x00, 0x08,
        0x69, 0x2d, 0x64, 0x65, 0x76, 0x69, 0x63, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    fn cfg() -> DeviceConfig {
        DeviceConfig {
            mac: MacAddr([0x02, 0xc0, 0xa8, 0x01, 0x0f, 0x02]),
            properties: DeviceProperties {
                name_of_station: "i-device".to_string(),
                type_of_station: "S7-1500 (PLCSIM)".to_string(),
                vendor_id: 0x002a,
                device_id: 0x010e,
                device_role: 0x0000,
                device_instance: 0x1064,
                device_options: vec![0x02, 0x07],
                ip: [192, 168, 1, 61],
                subnet: [255, 255, 255, 0],
                gateway: [192, 168, 1, 61],
                ip_block_info: 0x0001,
            },
        }
    }

    #[test]
    fn responds_to_matching_identify() {
        let resp = handle_dcp_frame(REQ_FRAME, &cfg())
            .unwrap()
            .expect("expected a response");
        // dst must be the requester (controller), src must be our device
        assert_eq!(&resp[0..6], &[0xec, 0x1c, 0x5d, 0x61, 0xe7, 0x3f]);
        assert_eq!(&resp[6..12], &[0x02, 0xc0, 0xa8, 0x01, 0x0f, 0x02]);
        assert_eq!(&resp[14..16], &[0xfe, 0xff]); // Identify response FrameID
                                                  // echoes the request Xid
        assert_eq!(&resp[18..22], &[0x03, 0x00, 0x01, 0x52]);
    }

    #[test]
    fn ignores_other_device_name() {
        let mut c = cfg();
        c.properties.name_of_station = "other".to_string();
        assert_eq!(handle_dcp_frame(REQ_FRAME, &c).unwrap(), None);
    }

    #[test]
    fn ignores_non_profinet_frame() {
        let mut f = REQ_FRAME.to_vec();
        f[12] = 0x08;
        f[13] = 0x00; // ethertype IPv4
        assert_eq!(handle_dcp_frame(&f, &cfg()).unwrap(), None);
    }
}
