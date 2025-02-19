/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::{self, Debug};
use std::io::{self, Cursor};
use std::pin::Pin;
use std::task::{Context, Poll};

use thrussh::client::Channel;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

pub struct Forward {
    writer: mpsc::UnboundedSender<Vec<u8>>,
    reader: mpsc::UnboundedReceiver<Vec<u8>>,
    read_buf: Vec<u8>,
}

impl Forward {
    pub fn new(channel: Channel) -> Self {
        let (writer, writer_recv) = mpsc::unbounded_channel();
        let (reader_send, reader) = mpsc::unbounded_channel();

        tokio::spawn(forward(channel, reader_send, writer_recv));

        Self {
            reader,
            writer,
            read_buf: Vec::with_capacity(1024),
        }
    }
}

async fn forward(
    mut channel: Channel,
    sender: mpsc::UnboundedSender<Vec<u8>>,
    mut receiver: mpsc::UnboundedReceiver<Vec<u8>>,
) {
    loop {
        tokio::select! {
            msg = channel.wait() => match msg {
            Some(thrussh::ChannelMsg::Data { data })
                | Some(thrussh::ChannelMsg::ExtendedData { data, .. }) => {
                if sender.send(data.to_vec()).is_err() {
                    break;
                }
                },
        Some(thrussh::ChannelMsg::Eof)
            | Some(thrussh::ChannelMsg::Close) => {
            eprintln!("SSH channel closed!");
            break;
            }
        Some(_) => continue,
            None => break
            },
            data = receiver.recv() => match data {
            Some(data) => {
                let mut cursor = Cursor::new(data.as_slice());
                while cursor.position() < data.len() as u64 {
                if channel.data(&mut cursor).await.is_err() {
                    break;
                }
                }
            },
            None => break
            }
        }
    }
}

impl Debug for Forward {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ssh::Forward {{ /* fields omitted */ }}")
    }
}

impl AsyncRead for Forward {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            if !self.read_buf.is_empty() {
                let n = self.read_buf.len().min(buf.remaining());
                buf.put_slice(&self.read_buf[0..n]);
                self.read_buf.drain(0..n);
                return Poll::Ready(Ok(()));
            }

            match self.reader.poll_recv(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Ready(Some(data)) => self.read_buf.extend(data),
            }
        }
    }
}

impl AsyncWrite for Forward {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.writer.send(buf.to_vec()) {
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e))),
            Ok(()) => Poll::Ready(Ok(buf.len())),
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
