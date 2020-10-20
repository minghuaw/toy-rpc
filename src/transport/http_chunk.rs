use std::pin::Pin;
use futures::task::{Context, Poll};
use futures::io::{AsyncRead, AsyncBufRead, AsyncWrite, Error};
use pin_project::pin_project;

#[pin_project]
pub struct Chunk<'a> {
    #[pin]
    pub req_buf: &'a [u8],
    #[pin]
    pub res_buf: &'a mut Vec<u8>
}

impl<'a> Chunk<'a> {
    pub fn new(req_buf: &'a [u8], res_buf: &'a mut Vec<u8>) -> Self {
        Self {
            req_buf,
            res_buf
        }
    }
}

impl<'a> AsyncRead for Chunk<'a> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, Error>>{
        let this = self.project();
        let req_buf = this.req_buf;
        req_buf.poll_read(cx, buf)
    }
}

impl<'a> AsyncBufRead for Chunk<'a> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8], Error>> {
        let this = self.project();
        let req_buf = this.req_buf;
        req_buf.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.project();
        let req_buf = this.req_buf;
        req_buf.consume(amt)
    }
}

impl<'a> AsyncWrite for Chunk<'a> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        let this = self.project();
        let res_buf = this.res_buf;
        res_buf.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = self.project();
        let res_buf = this.res_buf;
        res_buf.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let this = self.project();
        let res_buf = this.res_buf;
        res_buf.poll_close(cx)
    }
}


