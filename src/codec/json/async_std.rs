use cfg_if::cfg_if;

cfg_if!{
    if #[cfg(feature = "serde_bincode")] {

    } else if #[cfg(feature = "serde_cbor")] {

    } else if #[cfg(feature = "serde_rmp")] {

    } else { 
        use futures::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};
        
        use super::*;
        
        #[async_trait]
        impl<R, W> CodecRead for Codec<R, W, ConnTypeReadWrite>
        where
            R: AsyncBufRead + Send + Sync + Unpin,
            W: AsyncWrite + Send + Sync + Unpin,
        {
            async fn read_header<H>(&mut self) -> Option<Result<H, Error>>
            where
                H: serde::de::DeserializeOwned,
            {
                let mut buf = String::new();
                match self.reader.read_line(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            // EOF, probably end of connection
                            return None;
                        }
        
                        Some(Self::unmarshal(buf.as_bytes()))
                    }
                    Err(err) => {
                        log::error!("{:?}", err.kind());
                        Some(Err(err.into()))
                    }
                }
            }
        
            async fn read_body(
                &mut self,
            ) -> Option<Result<Box<dyn erased::Deserializer<'static> + Send + 'static>, Error>> {
                let mut buf = String::new();
        
                match self.reader.read_line(&mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            // EOF, probablen client closed connection
                            return None;
                        }
        
                        let de = Self::from_bytes(buf.into_bytes());
                        Some(Ok(de))
                    }
                    Err(err) => return Some(Err(err.into())),
                }
            }
        }
        
        #[async_trait]
        impl<R, W> CodecWrite for Codec<R, W, ConnTypeReadWrite>
        where
            R: AsyncBufRead + Send + Sync + Unpin,
            W: AsyncWrite + Send + Sync + Unpin,
        {
            async fn write_header<H>(&mut self, header: H) -> Result<(), Error>
            where
                H: serde::Serialize + Metadata + Send,
            {
                let id = header.get_id();
                let buf = Self::marshal(&header)?;
        
                let bytes_sent = self.writer.write(&buf).await?;
                self.writer.flush().await?;
        
                log::trace!("Header id: {} written with {} bytes", &id, &bytes_sent);
        
                Ok(())
            }
        
            async fn write_body(
                &mut self,
                id: &MessageId,
                body: &(dyn erased::Serialize + Send + Sync),
            ) -> Result<(), Error> {
                let buf = Self::marshal(&body)?;
        
                let bytes_sent = self.writer.write(&buf).await?;
                self.writer.flush().await?;
        
                log::trace!("Body id: {} written with {} bytes", id, &bytes_sent);
        
                Ok(())
            }
        }
    }
}

