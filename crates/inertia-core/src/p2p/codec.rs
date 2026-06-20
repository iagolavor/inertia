use std::io;
use std::time::Duration;

use async_trait::async_trait;
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;

use super::protocol::{InertiaRequest, InertiaResponse, PROTOCOL_NAME};

#[derive(Clone, Default)]
pub struct JsonCodec;

#[async_trait]
impl Codec for JsonCodec {
    type Protocol = StreamProtocol;
    type Request = InertiaRequest;
    type Response = InertiaResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            futures::AsyncReadExt::read(io, &mut byte).await?;
            if byte[0] == 0 {
                break;
            }
            buf.push(byte[0]);
        }
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            futures::AsyncReadExt::read(io, &mut byte).await?;
            if byte[0] == 0 {
                break;
            }
            buf.push(byte[0]);
        }
        serde_json::from_slice(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        futures::AsyncWriteExt::write_all(io, &data).await?;
        futures::AsyncWriteExt::write_all(io, &[0]).await?;
        futures::AsyncWriteExt::flush(io).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        futures::AsyncWriteExt::write_all(io, &data).await?;
        futures::AsyncWriteExt::write_all(io, &[0]).await?;
        futures::AsyncWriteExt::flush(io).await
    }
}

pub fn request_response_config() -> libp2p::request_response::Config {
    libp2p::request_response::Config::default().with_request_timeout(Duration::from_secs(90))
}

pub fn protocol_stream() -> StreamProtocol {
    StreamProtocol::new(PROTOCOL_NAME)
}
