use std::fs::File;
use std::io::Read;
use std::path::Path;

use pcap_file::pcap::PcapReader;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("io error: {0}")]
    Io(String),
    #[error("pcap parse error: {0}")]
    Pcap(String),
}

/// Itérateur sur les trames Ethernet brutes d'un fichier pcap.
pub struct PcapFrames<R: Read> {
    reader: PcapReader<R>,
}

impl PcapFrames<File> {
    pub fn open(path: &Path) -> Result<Self, CaptureError> {
        let file = File::open(path).map_err(|e| CaptureError::Io(e.to_string()))?;
        Self::from_reader(file)
    }
}

impl<R: Read> PcapFrames<R> {
    pub fn from_reader(r: R) -> Result<Self, CaptureError> {
        let reader = PcapReader::new(r).map_err(|e| CaptureError::Pcap(e.to_string()))?;
        Ok(Self { reader })
    }
}

impl<R: Read> Iterator for PcapFrames<R> {
    type Item = Vec<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next_packet() {
            Some(Ok(pkt)) => Some(pkt.data.into_owned()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pcap_file::pcap::{PcapPacket, PcapWriter};
    use std::io::Cursor;
    use std::time::Duration;

    fn make_pcap(frames: &[&[u8]]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = PcapWriter::new(&mut buf).unwrap();
            for f in frames {
                w.write_packet(&PcapPacket::new(Duration::ZERO, f.len() as u32, f))
                    .unwrap();
            }
        }
        buf
    }

    #[test]
    fn reads_all_frames_in_order() {
        let bytes = make_pcap(&[&[0xaa, 0xbb], &[0xcc]]);
        let frames: Vec<Vec<u8>> = PcapFrames::from_reader(Cursor::new(bytes))
            .unwrap()
            .collect();
        assert_eq!(frames, vec![vec![0xaa, 0xbb], vec![0xcc]]);
    }
}
