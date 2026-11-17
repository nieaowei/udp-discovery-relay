use std::net::Ipv4Addr;
/// Internet checksum, including an odd trailing byte.
pub fn checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    for pair in bytes.chunks(2) {
        sum += ((pair[0] as u32) << 8) | *pair.get(1).unwrap_or(&0) as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
/// Build an unfragmented IPv4 UDP datagram, preserving the sender's endpoint.
/// The buffer is reused by the event loop. UDP checksum covers the pseudoheader.
pub fn build(
    out: &mut Vec<u8>,
    src: Ipv4Addr,
    dst: Ipv4Addr,
    sport: u16,
    dport: u16,
    data: &[u8],
) -> bool {
    if data.len() > 65507 {
        return false;
    }
    out.clear();
    out.resize(28 + data.len(), 0);
    out[0] = 0x45;
    let len = out.len() as u16;
    out[2..4].copy_from_slice(&len.to_be_bytes());
    out[6] = 0x40; // DF: report an MTU failure instead of silently fragmenting.
    out[8] = 1;
    out[9] = 17;
    out[12..16].copy_from_slice(&src.octets());
    out[16..20].copy_from_slice(&dst.octets());
    out[20..22].copy_from_slice(&sport.to_be_bytes());
    out[22..24].copy_from_slice(&dport.to_be_bytes());
    let udp_len = (data.len() + 8) as u16;
    out[24..26].copy_from_slice(&udp_len.to_be_bytes());
    out[28..].copy_from_slice(data);
    // Reuse the IP header's last 12 bytes as pseudoheader temporarily.
    let saved: [u8; 12] = out[8..20].try_into().unwrap();
    out[8..12].copy_from_slice(&src.octets());
    out[12..16].copy_from_slice(&dst.octets());
    out[16] = 0;
    out[17] = 17;
    out[18..20].copy_from_slice(&udp_len.to_be_bytes());
    let c = checksum(&out[8..]);
    out[8..20].copy_from_slice(&saved);
    out[26..28].copy_from_slice(&(if c == 0 { 0xffff } else { c }).to_be_bytes());
    let c = checksum(&out[..20]);
    out[10..12].copy_from_slice(&c.to_be_bytes());
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_checksums_and_boundaries() {
        for n in [0, 1, 2, 513, 65507] {
            let mut p = Vec::new();
            let data = vec![0xa5; n];
            assert!(build(
                &mut p,
                Ipv4Addr::new(192, 168, 1, 2),
                Ipv4Addr::new(224, 0, 0, 167),
                1234,
                53317,
                &data
            ));
            assert_eq!(checksum(&p[..20]), 0);
            assert_eq!(&p[28..], &data);
            let mut pseudo = Vec::from(&p[12..20]);
            pseudo.extend([0, 17]);
            pseudo.extend_from_slice(&p[24..26]);
            pseudo.extend_from_slice(&p[20..]);
            assert_eq!(checksum(&pseudo), 0);
        }
        assert!(!build(
            &mut Vec::new(),
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::BROADCAST,
            1,
            1,
            &vec![0; 65508]
        ));
    }
}
