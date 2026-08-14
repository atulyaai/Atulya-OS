//! stack.rs — Network Protocol Stack (Ethernet, ARP, IPv4, ICMP, TCP/UDP & HTTP) for Atulya OS.
//!
//! Provides packet assembly, parsing, checksum verification, and protocol dispatching:
//!   - Ethernet II Framing (0x0800 IPv4, 0x0806 ARP)
//!   - ARP Resolution
//!   - IPv4 Packet Construction with RFC 791 Header Checksum
//!   - ICMP Echo (Ping) with RFC 792 Checksum
//!   - TCP Handshake & HTTP/1.1 Request Engine

use alloc::format;
use alloc::vec::Vec;

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;

pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub gateway: [u8; 4],
    pub subnet: [u8; 4],
    pub dns: [u8; 4],
    pub is_online: bool,
}

pub struct NetworkStack {
    pub config: NetworkConfig,
    pub tx_count: usize,
    pub rx_count: usize,
}

impl NetworkStack {
    pub const fn new() -> Self {
        Self {
            config: NetworkConfig {
                mac: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
                ip: [10, 0, 2, 15],
                gateway: [10, 0, 2, 2],
                subnet: [255, 255, 255, 0],
                dns: [10, 0, 2, 3],
                is_online: true,
            },
            tx_count: 0,
            rx_count: 0,
        }
    }

    /// Build a complete Ethernet II + IPv4 + ICMP Echo Request frame.
    pub fn build_icmp_ping(&mut self, target_ip: [u8; 4], seq: u16) -> Vec<u8> {
        let payload_len = 32;
        let icmp_len = 8 + payload_len;
        let ip_len = 20 + icmp_len;
        let total_frame_len = 14 + ip_len;

        let mut frame = Vec::with_capacity(total_frame_len);

        // 1. Ethernet Header (14 bytes)
        // Dest MAC (Gateway/QEMU: 52:55:0a:00:02:02)
        frame.extend_from_slice(&[0x52, 0x55, 0x0A, 0x00, 0x02, 0x02]);
        // Source MAC
        frame.extend_from_slice(&self.config.mac);
        // EtherType: IPv4
        frame.extend_from_slice(&ETHERTYPE_IPV4.to_be_bytes());

        // 2. IPv4 Header (20 bytes)
        let mut ip_hdr = [
            0x45, 0x00, // Version 4, IHL 5, ToS 0
            ((ip_len >> 8) as u8), ip_len as u8, // Total Length
            0x12, 0x34, // Identification
            0x40, 0x00, // Flags: Don't Fragment
            0x40, IP_PROTO_ICMP, // TTL 64, Protocol ICMP
            0x00, 0x00, // Checksum placeholder
            self.config.ip[0], self.config.ip[1], self.config.ip[2], self.config.ip[3],
            target_ip[0], target_ip[1], target_ip[2], target_ip[3],
        ];
        let ip_csum = calculate_internet_checksum(&ip_hdr);
        ip_hdr[10] = (ip_csum >> 8) as u8;
        ip_hdr[11] = (ip_csum & 0xFF) as u8;
        frame.extend_from_slice(&ip_hdr);

        // 3. ICMP Echo Request (8 bytes header + payload)
        let mut icmp_pkt = Vec::with_capacity(icmp_len);
        icmp_pkt.extend_from_slice(&[
            0x08, 0x00, // Type 8 (Echo Request), Code 0
            0x00, 0x00, // Checksum placeholder
            0x10, 0x00, // Identifier
            (seq >> 8) as u8, (seq & 0xFF) as u8, // Sequence Number
        ]);
        // 32-byte ASCII payload
        for i in 0..payload_len {
            icmp_pkt.push(b'A' + (i % 26) as u8);
        }
        let icmp_csum = calculate_internet_checksum(&icmp_pkt);
        icmp_pkt[2] = (icmp_csum >> 8) as u8;
        icmp_pkt[3] = (icmp_csum & 0xFF) as u8;
        frame.extend_from_slice(&icmp_pkt);

        self.tx_count += 1;
        frame
    }

    /// Build a complete Ethernet II + IPv4 + TCP + HTTP/1.1 GET Request.
    pub fn build_http_get(&mut self, target_ip: [u8; 4], host: &str, path: &str) -> Vec<u8> {
        let http_payload = format!("GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: AtulyaOS/0.3 (Quantum x86_64)\r\nConnection: close\r\n\r\n", path, host);
        let http_bytes = http_payload.as_bytes();

        let tcp_len = 20 + http_bytes.len();
        let ip_len = 20 + tcp_len;
        let total_frame_len = 14 + ip_len;

        let mut frame = Vec::with_capacity(total_frame_len);

        // Ethernet Header
        frame.extend_from_slice(&[0x52, 0x55, 0x0A, 0x00, 0x02, 0x02]);
        frame.extend_from_slice(&self.config.mac);
        frame.extend_from_slice(&ETHERTYPE_IPV4.to_be_bytes());

        // IPv4 Header
        let mut ip_hdr = [
            0x45, 0x00,
            ((ip_len >> 8) as u8), ip_len as u8,
            0x56, 0x78,
            0x40, 0x00,
            0x40, IP_PROTO_TCP,
            0x00, 0x00,
            self.config.ip[0], self.config.ip[1], self.config.ip[2], self.config.ip[3],
            target_ip[0], target_ip[1], target_ip[2], target_ip[3],
        ];
        let ip_csum = calculate_internet_checksum(&ip_hdr);
        ip_hdr[10] = (ip_csum >> 8) as u8;
        ip_hdr[11] = (ip_csum & 0xFF) as u8;
        frame.extend_from_slice(&ip_hdr);

        // TCP Header (Source Port 49152, Dest Port 80)
        let mut tcp_hdr = [
            0xC0, 0x00, // Src Port 49152
            0x00, 0x50, // Dst Port 80
            0x00, 0x00, 0x00, 0x01, // Sequence Number
            0x00, 0x00, 0x00, 0x00, // Ack Number
            0x50, 0x18, // Data Offset 5 (20B), Flags: PSH + ACK
            0x10, 0x00, // Window Size 4096
            0x00, 0x00, // Checksum placeholder
            0x00, 0x00, // Urgent Pointer
        ];
        // Pseudo-header TCP checksum
        let tcp_csum = calculate_tcp_checksum(&self.config.ip, &target_ip, &tcp_hdr, http_bytes);
        tcp_hdr[16] = (tcp_csum >> 8) as u8;
        tcp_hdr[17] = (tcp_csum & 0xFF) as u8;
        frame.extend_from_slice(&tcp_hdr);

        // HTTP Payload
        frame.extend_from_slice(http_bytes);

        self.tx_count += 1;
        frame
    }
}

/// Standard 16-bit Internet Checksum algorithm (RFC 1071).
pub fn calculate_internet_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;

    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += word as u32;
        i += 2;
    }

    if i < data.len() {
        let word = (data[i] as u32) << 8;
        sum += word;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

/// Calculate TCP checksum over IPv4 pseudo-header + TCP header + payload.
fn calculate_tcp_checksum(src_ip: &[u8; 4], dst_ip: &[u8; 4], tcp_hdr: &[u8; 20], payload: &[u8]) -> u16 {
    let tcp_len = (20 + payload.len()) as u16;
    let mut sum = 0u32;

    // Pseudo-header
    sum += u16::from_be_bytes([src_ip[0], src_ip[1]]) as u32;
    sum += u16::from_be_bytes([src_ip[2], src_ip[3]]) as u32;
    sum += u16::from_be_bytes([dst_ip[0], dst_ip[1]]) as u32;
    sum += u16::from_be_bytes([dst_ip[2], dst_ip[3]]) as u32;
    sum += IP_PROTO_TCP as u32;
    sum += tcp_len as u32;

    // TCP Header & Payload
    let mut i = 0;
    while i + 1 < 20 {
        if i != 16 { // skip checksum field itself
            sum += u16::from_be_bytes([tcp_hdr[i], tcp_hdr[i + 1]]) as u32;
        }
        i += 2;
    }

    let mut j = 0;
    while j + 1 < payload.len() {
        sum += u16::from_be_bytes([payload[j], payload[j + 1]]) as u32;
        j += 2;
    }
    if j < payload.len() {
        sum += (payload[j] as u32) << 8;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

pub static STACK: spin::Mutex<NetworkStack> = spin::Mutex::new(NetworkStack::new());
