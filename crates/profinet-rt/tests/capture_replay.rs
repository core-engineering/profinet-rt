//! Test d'intégration : rejoue un pcap fixture s'il existe (sinon ignoré).
use profinet_rt::capture::PcapFrames;
use std::path::Path;

#[test]
fn replay_fixture_if_present() {
    let p = Path::new("tests/fixtures/sample.pcap");
    if !p.exists() {
        eprintln!("pas de fixture sample.pcap — test ignoré");
        return;
    }
    let frames: Vec<Vec<u8>> = PcapFrames::open(p)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let n = frames.len();
    assert!(n > 0, "le pcap fixture ne doit pas être vide");
}
