use uuid::Uuid;

use super::{InsufficientBuffer, ProxyAddressWithPort, VlessHeaderParseError};
use crate::{BufferFormer, BufferParseResult, BufferParser};

#[derive(Debug)]
pub struct VlessRequestHeader<'a> {
    pub address: ProxyAddressWithPort<'a>,
    pub user: Uuid,
    pub command: VlessCommand,
}

#[derive(Debug)]
pub enum VlessCommand {
    Mux,
    Tcp,
    Udp,
}

#[derive(Copy, Clone, Default)]
pub struct VlessrParseOptions {
    is_fb: bool,
}

impl<'a> BufferParser<'a> for VlessRequestHeader<'a> {
    type Error = VlessHeaderParseError;
    type ParseOptions = VlessrParseOptions;

    fn parse_with_options<'b>(
        buffer: &'b [u8],
        options: VlessrParseOptions,
    ) -> BufferParseResult<Self, Self::Error>
    where
        Self: Sized,
        'b: 'a,
    {
        let min_size_before_cmd = (if options.is_fb { 2 } else { 1 }) * 17;
        let min_size = min_size_before_cmd + 1;
        if buffer.len() < min_size {
            return BufferParseResult::Incomplete {
                needed: min_size - buffer.len() + 1,
            };
        }
        if buffer[0] != 0x00 {
            return BufferParseResult::Error(VlessHeaderParseError::InvalidVersion);
        }
        let user = uuid::Builder::from_slice(&buffer[1..17])
            .unwrap()
            .into_uuid();
        if buffer[17] != 0x00 {
            return BufferParseResult::Error(VlessHeaderParseError::AddonIsNotSupported);
        }

        let (command, address) = match buffer[min_size] {
            0x01 => (
                VlessCommand::Tcp,
                ProxyAddressWithPort::parse(&buffer[min_size + 1..]),
            ),
            0x02 => (
                VlessCommand::Udp,
                ProxyAddressWithPort::parse(&buffer[min_size + 1..]),
            ),
            0x03 => (
                VlessCommand::Mux,
                BufferParseResult::Parsed {
                    value: ProxyAddressWithPort {
                        address: super::ProxyAddress::Domain("v1.mux.cool"),
                        port: 0,
                    },
                    size: 0,
                },
            ),
            _ => return BufferParseResult::Error(VlessHeaderParseError::InvalidCommand),
        };
        match address {
            BufferParseResult::Parsed {
                value: address,
                size,
            } => BufferParseResult::Parsed {
                value: VlessRequestHeader {
                    address,
                    user,
                    command,
                },
                size: min_size + 1 + size,
            },
            BufferParseResult::Incomplete { needed } => BufferParseResult::Incomplete { needed },
            BufferParseResult::Error(_) => {
                BufferParseResult::Error(VlessHeaderParseError::InvalidAddress)
            }
        }
    }
}

impl<'a> BufferFormer for VlessRequestHeader<'a> {
    type Error = InsufficientBuffer;
    type FormingOptions = ();

    fn size_with_option(&self, _: &Self::FormingOptions) -> usize {
        18 + 1 + self.address.size()
    }

    fn form_with_option<'b>(
        &'b self,
        buffer: &'b mut [u8],
        _: &Self::FormingOptions,
    ) -> Result<usize, Self::Error> {
        if buffer.len() < self.size() {
            return Err(InsufficientBuffer);
        }
        buffer[0] = 0x00;
        buffer[1..17].copy_from_slice(&*self.user.as_bytes());
        buffer[17] = 0x00;
        buffer[18] = match self.command {
            VlessCommand::Tcp => 0x01,
            VlessCommand::Udp => 0x02,
            VlessCommand::Mux => 0x03,
        };
        self.address.form(&mut buffer[19..]).map(|size| 19 + size)
    }
}
