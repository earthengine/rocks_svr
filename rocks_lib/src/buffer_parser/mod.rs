// This module defines a parsing framework for buffers in Rust. It includes the
// `BufferParseResult` enum to represent the outcome of a parsing operation,
// which can be a successful parse, an incomplete parse, or an error. The
// `BufferParser` trait is designed for types that can parse byte buffers,
// providing methods for parsing with options and a default parsing method.
// Additionally, the `BufferFormer` trait is defined for types that can form
// byte buffers, specifying methods to determine the size of the data and to
// populate a mutable buffer with data.

use std::net::SocketAddr;

use anyhow::Error;
use futures::{Sink, Stream};
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
pub enum BufferParseResult<T, E> {
    Parsed { value: T, size: usize },
    Incomplete { needed: usize },
    Error(E),
}

pub trait BufferParser<'a> {
    type Error;
    type ParseOptions: Clone + Default;

    /// Parse a buffer with the given options.
    /// This is the required method to implement for a buffer parser.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to parse.
    /// * `options` - The options to use for parsing.
    /// * 'b - The lifetime of the buffer, will outlive the parsed result.
    ///
    /// # Returns
    ///
    /// A `BufferParseResult` representing the outcome of the parse operation.
    /// When completed, the pared result is returned with the parsed value and the size of the parsed data advanced.
    /// When incomplete, the needed (minimum) extra data size in bytes is returned.
    /// It may also return an error if the parsing fails.
    ///
    fn parse_with_options<'b>(
        buffer: &'b [u8],
        options: Self::ParseOptions,
    ) -> BufferParseResult<Self, Self::Error>
    where
        Self: Sized,
        'b: 'a;

    /// Parse a buffer with default options.
    ///
    fn parse(buffer: &'a [u8]) -> BufferParseResult<Self, Self::Error>
    where
        Self: Sized,
    {
        Self::parse_with_options(buffer, Self::ParseOptions::default())
    }
}

pub trait BufferFormer {
    type Error;
    type FormingOptions: Clone + Default;

    /// Get the size of the data that will be formed.
    fn size_with_option(&self, options: &Self::FormingOptions) -> usize;
    fn size(&self) -> usize {
        self.size_with_option(&Self::FormingOptions::default())
    }

    /// Form the data into the given buffer.
    fn form_with_option<'a>(
        &'a self,
        buffer: &'a mut [u8],
        options: &Self::FormingOptions,
    ) -> Result<usize, Self::Error>;
    fn form<'a>(&'a self, buffer: &'a mut [u8]) -> Result<usize, Self::Error> {
        self.form_with_option(buffer, &Self::FormingOptions::default())
    }
}

#[trait_variant::make(Protocol: Send + Sync)]
#[allow(dead_code)]
pub(crate) trait LocalProtocol {
    async fn handle(
        &self,
        connection: impl AsyncRead + AsyncWrite + Send + Sync + Unpin,
        remote_addr: SocketAddr,
    ) -> Result<(), Error>;
}

#[trait_variant::make(WsProtocol: Send + Sync)]
#[allow(dead_code)]
pub(crate) trait WsLocalProtocol {
    async fn handle(
        &self,
        data_sink: impl Sink<Vec<u8>> + Send + Sync + Unpin,
        data_stream: impl Stream<Item = Vec<u8>> + Send + Sync + Unpin,
        remote_addr: SocketAddr,
    ) -> Result<(), Error>;
}
