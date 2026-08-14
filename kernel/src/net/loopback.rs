use alloc::vec::Vec;

pub struct LoopbackInterface {
    rx_queue: alloc::collections::VecDeque<Vec<u8>>,
    tx_queue: alloc::collections::VecDeque<Vec<u8>>,
    ip_addr: [u8; 4],
}

impl LoopbackInterface {
    pub fn new() -> Self {
        let rx_queue = alloc::collections::VecDeque::new();
        let tx_queue = alloc::collections::VecDeque::new();

        Self { rx_queue, tx_queue, ip_addr: [127, 0, 0, 1] }
    }

    pub fn inject_packet(&mut self, data: &[u8]) {
        self.rx_queue.push_back(data.to_vec());
    }

    pub fn transmitted_packets(&mut self) -> Option<Vec<u8>> {
        self.tx_queue.pop_front()
    }

    pub fn ip_address(&self) -> [u8; 4] {
        self.ip_addr
    }

    pub fn send_loopback(&mut self, data: &[u8]) {
        self.rx_queue.push_back(data.to_vec());
    }
}
