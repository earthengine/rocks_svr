use crate::buffer_parser::{BufferFormer, BufferParseResult, BufferParser};
use derive_more::derive::Display;
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::net::lookup_host;

#[derive(Debug, Display)]
pub enum ProxyAddress<'a> {
    IPv4(Ipv4Addr),
    Domain(&'a str),
    IPv6(Ipv6Addr),
}

pub struct ProxyAddressParser;

#[derive(Debug, PartialEq)]
pub struct InvalidAddressType;
#[derive(Debug, PartialEq)]
pub struct InsufficientBuffer;

impl<'a> BufferParser<'a> for ProxyAddress<'a> {
    type Error = InvalidAddressType;
    type ParseOptions = ();

    fn parse_with_options<'b>(
        buffer: &'b [u8],
        _: (),
    ) -> BufferParseResult<Self, InvalidAddressType>
    where
        Self: Sized,
        'b: 'a,
    {
        if buffer.len() < 3 {
            return BufferParseResult::Incomplete {
                needed: 3 - buffer.len(),
            };
        }
        match buffer[0] {
            0x01 => {
                if buffer.len() < 5 {
                    return BufferParseResult::Incomplete {
                        needed: 5 - buffer.len(),
                    };
                }
                let ip = Ipv4Addr::from_bits(u32::from_be_bytes(buffer[1..5].try_into().unwrap()));
                BufferParseResult::Parsed {
                    value: ProxyAddress::IPv4(ip),
                    size: 5,
                }
            }
            0x02 => {
                let domain_len = buffer[1] as usize;
                if buffer.len() < 2 + domain_len {
                    return BufferParseResult::Incomplete {
                        needed: 2 + domain_len - buffer.len(),
                    };
                }
                let domain = std::str::from_utf8(&buffer[2..2 + domain_len]).unwrap();
                BufferParseResult::Parsed {
                    value: ProxyAddress::Domain(domain),
                    size: 2 + domain_len,
                }
            }
            0x03 => {
                if buffer.len() < 17 {
                    return BufferParseResult::Incomplete {
                        needed: 17 - buffer.len(),
                    };
                }
                let ip =
                    Ipv6Addr::from_bits(u128::from_be_bytes(buffer[1..17].try_into().unwrap()));
                BufferParseResult::Parsed {
                    value: ProxyAddress::IPv6(ip),
                    size: 17,
                }
            }
            _ => BufferParseResult::Error(InvalidAddressType),
        }
    }
}

impl<'a> BufferFormer for ProxyAddress<'a> {
    type Error = InsufficientBuffer;
    type FormingOptions = ();

    fn form_with_option<'b>(
        &'b self,
        buffer: &'b mut [u8],
        _: &Self::FormingOptions,
    ) -> Result<usize, Self::Error> {
        match self {
            ProxyAddress::IPv4(ip) => {
                if buffer.len() < 5 {
                    return Err(InsufficientBuffer);
                }
                buffer[0] = 0x01;
                buffer[1..5].copy_from_slice(&ip.octets());
                Ok(5)
            }
            ProxyAddress::Domain(domain) => {
                let domain_len = domain.len();
                if buffer.len() < 2 + domain_len {
                    return Err(InsufficientBuffer);
                }
                buffer[0] = 0x02;
                buffer[1] = domain_len as u8;
                buffer[2..2 + domain_len].copy_from_slice(domain.as_bytes());
                Ok(2 + domain_len)
            }
            ProxyAddress::IPv6(ip) => {
                if buffer.len() < 17 {
                    return Err(InsufficientBuffer);
                }
                buffer[0] = 0x03;
                buffer[1..17].copy_from_slice(&ip.octets());
                Ok(17)
            }
        }
    }

    fn size_with_option(&self, _: &Self::FormingOptions) -> usize {
        match self {
            ProxyAddress::IPv4(_) => 5,
            ProxyAddress::Domain(domain) => 2 + domain.len(),
            ProxyAddress::IPv6(_) => 17,
        }
    }
}

#[derive(Debug, Display)]
#[display("{}:{}", address, port)]
pub struct ProxyAddressWithPort<'a> {
    pub address: ProxyAddress<'a>,
    pub port: u16,
}

impl<'a> ProxyAddressWithPort<'a> {
    pub async fn lookup_host(&self) -> Result<Vec<std::net::SocketAddr>, std::io::Error> {
        match &self.address {
            ProxyAddress::IPv4(ip) => Ok(lookup_host((*ip, self.port)).await?.collect()),
            ProxyAddress::IPv6(ip) => Ok(lookup_host((*ip, self.port)).await?.collect()),
            ProxyAddress::Domain(domain) => Ok(lookup_host((*domain, self.port)).await?.collect()),
        }
    }
}

impl<'a> BufferParser<'a> for ProxyAddressWithPort<'a> {
    type Error = InvalidAddressType;
    type ParseOptions = ();

    fn parse_with_options<'b>(
        buffer: &'b [u8],
        _: (),
    ) -> BufferParseResult<Self, InvalidAddressType>
    where
        Self: Sized,
        'b: 'a,
    {
        if buffer.len() < 3 {
            return BufferParseResult::Incomplete {
                needed: 3 - buffer.len(),
            };
        }
        let port: u16 = u16::from_be_bytes(buffer[..2].try_into().unwrap());
        let address = ProxyAddress::parse(buffer[2..].into());
        match address {
            BufferParseResult::Parsed {
                value: address,
                size,
            } => BufferParseResult::Parsed {
                value: ProxyAddressWithPort { address, port },
                size: size + 2,
            },
            BufferParseResult::Incomplete { needed } => BufferParseResult::Incomplete { needed },
            BufferParseResult::Error(e) => BufferParseResult::Error(e),
        }
    }
}

impl<'a> BufferFormer for ProxyAddressWithPort<'a> {
    type Error = InsufficientBuffer;
    type FormingOptions = ();

    fn form_with_option<'b>(
        &'b self,
        buffer: &'b mut [u8],
        _: &Self::FormingOptions,
    ) -> Result<usize, Self::Error> {
        if buffer.len() < 2 {
            return Err(InsufficientBuffer);
        }
        buffer[..2].copy_from_slice(&self.port.to_be_bytes());
        let address_size = self.address.form(&mut buffer[2..])?;
        Ok(2 + address_size)
    }

    fn size_with_option(&self, _: &Self::FormingOptions) -> usize {
        2 + self.address.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    #[test]
    fn test_parse_domain() {
        let buffer = [
            0x02, 0x0B, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',
        ];
        match ProxyAddress::parse(&buffer) {
            BufferParseResult::Parsed {
                value: ProxyAddress::Domain(domain),
                ..
            } => {
                assert_eq!(domain, "example.com");
            }
            _ => panic!("Failed to parse domain"),
        }
    }

    #[test]
    fn test_parse_ipv6() {
        let buffer = [
            0x03, 0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
            0x70, 0x73, 0x34,
        ];

        let expected_ip = Ipv6Addr::new(0x2001, 0xdb8, 0x85a3, 0x0, 0x0, 0x8a2e, 0x370, 0x7334);
        match ProxyAddress::parse(&buffer) {
            BufferParseResult::Parsed {
                value: ProxyAddress::IPv6(ip),
                ..
            } => {
                assert_eq!(ip, expected_ip);
            }
            _ => panic!("Failed to parse IPv6 address"),
        }
    }

    #[test]
    fn test_incomplete_domain() {
        let buffer = [0x02, 0x0B, b'e', b'x', b'a', b'm', b'p', b'l', b'e'];
        match ProxyAddress::parse(&buffer) {
            BufferParseResult::Incomplete { needed } => {
                assert_eq!(needed, 4);
            }
            _ => panic!("Expected incomplete buffer"),
        }
    }

    #[test]
    fn test_incomplete_ipv6() {
        let buffer = [
            0x03, 0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
            0x70,
        ];
        match ProxyAddress::parse(&buffer) {
            BufferParseResult::Incomplete { needed } => {
                assert_eq!(needed, 2);
            }
            _ => panic!("Expected incomplete buffer"),
        }
    }

    #[test]
    fn test_invalid_type() {
        let buffer = [0x04, 0, 0];
        match ProxyAddress::parse(&buffer) {
            BufferParseResult::Error(InvalidAddressType) => (),
            _ => panic!("Expected error for invalid type"),
        }
    }

    #[test]
    fn test_form_buffer_with_ipv4() {
        let address = ProxyAddress::IPv4(Ipv4Addr::new(192, 168, 1, 1));
        let mut buffer = vec![0x00; address.size()];
        assert_eq!(address.form(&mut buffer).unwrap(), 5);
        assert_eq!(buffer, vec![0x01, 192, 168, 1, 1]);
    }

    #[test]
    fn test_form_buffer_with_ipv6() {
        let address = ProxyAddress::IPv6(Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0, 0x0, 0x8a2e, 0x370, 0x7334,
        ));
        let mut buffer = vec![0x00; address.size()];
        assert_eq!(address.form(&mut buffer).unwrap(), 17);
        assert_eq!(
            buffer,
            vec![
                0x03, 0x20, 0x01, 0x0d, 0xb8, 0x85, 0xa3, 0x00, 0x00, 0x00, 0x00, 0x8a, 0x2e, 0x03,
                0x70, 0x73, 0x34
            ]
        );
    }

    #[test]
    fn test_form_buffer_with_domain() {
        let address = ProxyAddress::Domain("example.com");
        let mut buffer = vec![0x00; address.size()];
        assert_eq!(address.form(&mut buffer).unwrap(), 13);
        assert_eq!(
            buffer,
            vec![0x02, 0x0B, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm']
        );
    }

    #[test]
    fn test_form_buffer_with_empty_domain() {
        let address = ProxyAddress::Domain("");
        let mut buffer = vec![0x00; address.size()];
        assert_eq!(address.form(&mut buffer).unwrap(), 2);
        assert_eq!(buffer, vec![0x02, 0x00]);
    }

    #[test]
    fn test_form_buffer_with_insufficient_buffer() {
        let address = ProxyAddress::IPv4(Ipv4Addr::new(192, 168, 1, 1));
        let mut buffer = Vec::with_capacity(2); // Intentionally small buffer
        let result = address.form(&mut buffer);
        assert_eq!(result, Err(InsufficientBuffer));
    }

    #[test]
    fn test_form_buffer_with_insufficient_buffer_ipv6() {
        let address = ProxyAddress::IPv6(Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0, 0x0, 0x8a2e, 0x370, 0x7334,
        ));
        let mut buffer = Vec::with_capacity(10); // Intentionally small buffer
        let result = address.form(&mut buffer);
        assert_eq!(result, Err(InsufficientBuffer));
    }

    #[test]
    fn test_form_buffer_with_insufficient_buffer_domain() {
        let address = ProxyAddress::Domain("example.com");
        let mut buffer = Vec::with_capacity(5); // Intentionally small buffer
        let result = address.form(&mut buffer);
        assert_eq!(result, Err(InsufficientBuffer));
    }
}
