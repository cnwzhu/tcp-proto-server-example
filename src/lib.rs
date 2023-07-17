use std::future::Future;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use futures_util::TryFutureExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tracing::debug;
use crate::codec::LengthDelimitedCodec;
use crate::command::Command;

use crate::protocol::Protocol;
use crate::session::SessionManager;

pub mod session;
mod error;
mod protocol;
mod message;
mod codec;
mod command;


pub async fn serve(
    addr: &str,
    session_mgr: Arc<impl SessionManager>,
) -> error::Result<()> {
    let listener = TcpListener::bind(addr).await.unwrap();
    tracing::info!("Server Listening at {}", addr);
    loop {
        let session_mgr = session_mgr.clone();
        let conn_ret = listener.accept().await;
        match conn_ret {
            Ok((stream, peer_addr)) => {
                tracing::info!("New connection: {}", peer_addr);
                stream.set_nodelay(true)?;
                let fut = handle_connection(stream, session_mgr);
                tokio::spawn(fut.inspect_err(|e| {
                    debug!("Connection error: {}", e);
                }));
            }

            Err(e) => {
                tracing::error!("Connection failure: {}", e);
            }
        }
    }
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn handle_connection<S, SM>(
    stream: S,
    session_mgr: Arc<SM>,
) -> impl Future<Output=Result<(), error::Error>>
    where
        S: AsyncWrite + AsyncRead + Unpin,
        SM: SessionManager,
{
    let (tx,mut rx) = mpsc::channel::<Box<dyn Command>>(1);
    let mut proto  = Protocol::new(stream, LengthDelimitedCodec, session_mgr, tx);
    proto.create_client_conn();
    async {
        loop {
            tokio::select! {
                msg = proto.read_message() => {
                    if let Some(msg) = msg? {
                        proto.process(msg).await;
                    } else {
                        break;
                    }
                }
                Some(msg) = rx.recv() => {
                    proto.command(msg).await;
                }
            }
        }
        tracing::info!("Connection closed");
        Ok(())
    }
}