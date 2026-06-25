use std::sync::Mutex;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(String),
}

/// Abstraction d'E/S de trames Ethernet brutes (en-tête L2 inclus).
pub trait EthTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError>;
    /// Renvoie `Ok(None)` si aucune trame n'est disponible avant `timeout`.
    fn recv(&self, timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError>;
}

/// Transport en mémoire pour les tests.
#[derive(Default)]
pub struct MockTransport {
    tx: Mutex<Vec<Vec<u8>>>,
    rx: Mutex<std::collections::VecDeque<Vec<u8>>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self::default()
    }
    /// Empile une trame que `recv` retournera ensuite (FIFO).
    pub fn push_rx(&self, frame: Vec<u8>) {
        self.rx.lock().unwrap().push_back(frame);
    }
    /// Toutes les trames émises via `send`, dans l'ordre.
    pub fn sent(&self) -> Vec<Vec<u8>> {
        self.tx.lock().unwrap().clone()
    }
}

impl EthTransport for MockTransport {
    fn send(&self, frame: &[u8]) -> Result<(), TransportError> {
        self.tx.lock().unwrap().push(frame.to_vec());
        Ok(())
    }
    fn recv(&self, _timeout: Option<Duration>) -> Result<Option<Vec<u8>>, TransportError> {
        Ok(self.rx.lock().unwrap().pop_front())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_records_sent_frames() {
        let t = MockTransport::new();
        t.send(&[1, 2, 3]).unwrap();
        t.send(&[4, 5]).unwrap();
        assert_eq!(t.sent(), vec![vec![1, 2, 3], vec![4, 5]]);
    }

    #[test]
    fn mock_returns_pushed_rx_in_order_then_none() {
        let t = MockTransport::new();
        t.push_rx(vec![9, 9]);
        assert_eq!(t.recv(None).unwrap(), Some(vec![9, 9]));
        assert_eq!(t.recv(None).unwrap(), None);
    }
}
