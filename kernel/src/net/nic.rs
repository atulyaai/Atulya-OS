use alloc::vec::Vec;
use alloc::string::String;

pub trait NetworkInterface {
    fn name(&self) -> &str;
    fn mac_address(&self) -> [u8; 6];
    fn ip_address(&self) -> [u8; 4];
    fn send(&mut self, data: &[u8]) -> Result<(), &'static str>;
    fn receive(&mut self) -> Option<Vec<u8>>;
    fn is_up(&self) -> bool;
}

pub struct VirtIONet {
    name: String,
    mac: [u8; 6],
    ip: [u8; 4],
    rx_buf: alloc::collections::VecDeque<Vec<u8>>,
    tx_buf: alloc::collections::VecDeque<Vec<u8>>,
    mmio_base: u64,
    initialized: bool,
}

impl VirtIONet {
    pub fn new(name: &str, mmio_base: u64) -> Self {
        Self {
            name: String::from(name),
            mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
            ip: [0, 0, 0, 0],
            rx_buf: alloc::collections::VecDeque::new(),
            tx_buf: alloc::collections::VecDeque::new(),
            mmio_base,
            initialized: false,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        if self.mmio_base == 0 {
            return Err("No MMIO base address configured");
        }
        self.initialized = true;
        Ok(())
    }

    pub fn set_ip(&mut self, a: u8, b: u8, c: u8, d: u8) {
        self.ip = [a, b, c, d];
    }
}

impl NetworkInterface for VirtIONet {
    fn name(&self) -> &str {
        &self.name
    }

    fn mac_address(&self) -> [u8; 6] {
        self.mac
    }

    fn ip_address(&self) -> [u8; 4] {
        self.ip
    }

    fn send(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("NIC not initialized");
        }
        self.tx_buf.push_back(data.to_vec());
        Ok(())
    }

    fn receive(&mut self) -> Option<Vec<u8>> {
        self.rx_buf.pop_front()
    }

    fn is_up(&self) -> bool {
        self.initialized
    }
}
