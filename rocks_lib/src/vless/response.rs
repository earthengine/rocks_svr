use thiserror::Error;

use crate::{BufferFormer, BufferParseResult, BufferParser};

use super::VlessHeaderParseError;

#[derive(Debug)]
pub struct VlessResponseHeader {}

impl<'a> BufferParser<'a> for VlessResponseHeader {
    type Error = VlessHeaderParseError;
    type ParseOptions = ();

    fn parse_with_options<'b>(buffer: &'b [u8], _: ()) -> BufferParseResult<Self, Self::Error>
    where
        Self: Sized,
        'b: 'a,
    {
        if buffer.len() < 2 {
            return BufferParseResult::Incomplete {
                needed: 2 - buffer.len(),
            };
        }
        if buffer[0] != 0x00 {
            return BufferParseResult::Error(VlessHeaderParseError::InvalidVersion);
        }
        if (buffer[1]) != 0x00 {
            return BufferParseResult::Error(VlessHeaderParseError::AddonIsNotSupported);
        }
        return BufferParseResult::Parsed {
            value: VlessResponseHeader {},
            size: 2,
        };
    }
}

#[derive(Debug, Error)]
pub enum Never {}

impl BufferFormer for VlessResponseHeader {
    type Error = Never;
    type FormingOptions = ();

    fn size_with_option(&self, _: &Self::FormingOptions) -> usize {
        2
    }

    fn form_with_option<'a>(
        &'a self,
        buffer: &'a mut [u8],
        _: &Self::FormingOptions,
    ) -> Result<usize, Never> {
        buffer[0] = 0x00;
        buffer[1] = 0x00;
        Ok(2)
    }
}
