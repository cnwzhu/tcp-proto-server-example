use std::sync::Arc;

use futures_util::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::codec::LengthDelimitedCodec;
use crate::command::Command;
use crate::error;
use crate::message::RequestMessage;
use crate::protocol::ProtocolState::Authenticated;
use crate::session::{Session, SessionManager};

pub struct Protocol<S, D, SM>
    where
        SM: SessionManager,
{
    stream: Stream<S, D>,
    id: String,
    state: ProtocolState,
    is_terminate: bool,
    session_mgr: Arc<SM>,
    session: Option<Arc<SM::Session>>,
    send: mpsc::Sender<Box<dyn Command>>,
}

impl<S, SM> Protocol<S, LengthDelimitedCodec, SM> where
    S: AsyncWrite + AsyncRead + Unpin,
    SM: SessionManager,
{
    pub(crate) fn new(stream: S,
                      decode: LengthDelimitedCodec,
                      session_mgr: Arc<SM>,
                      send: mpsc::Sender<Box<dyn Command>>,
    ) -> Self {
        let framed = Framed::new(stream, decode);
        Protocol {
            id: Uuid::new_v4().to_string(),
            stream: Stream::new(framed),
            state: ProtocolState::Unauthenticated,
            is_terminate: false,
            session_mgr,
            session: None,
            send,
        }
    }

    pub(crate) fn create_client_conn(&self) {
        self.session_mgr.new_client(self.id.clone());
    }

    pub(crate) async fn read_message(&mut self) -> error::Result<Option<RequestMessage>> {
        self.stream.read().await
    }

    pub(crate) async fn process(&mut self, msg: RequestMessage) -> bool {
        self.do_process(msg).await || self.is_terminate
    }

    pub(crate) async fn command(&mut self, cmd: Box<dyn Command>) {}

    async fn do_process(&mut self, msg: RequestMessage) -> bool {
        match msg {
            RequestMessage::Auth(auth) => {
                if self.state == Authenticated {
                    tracing::warn!("Already authenticated");
                }
                self.state = Authenticated;
                let (client_id, connect_code) = (self.id.clone(), auth.connect_code.clone());
                self.session = Some(self.session_mgr.create_session(client_id, connect_code, self.send.clone()));
            }
            RequestMessage::HeartBeat(_) => {}
        }
        false
    }
}

impl<S, D, SM> Drop for Protocol<S, D, SM>
    where
        SM: SessionManager,
{
    fn drop(&mut self) {
        if self.session.is_none() {
            return;
        }
        self.session_mgr.remove_session(self.session.as_ref().unwrap().client_id());
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ProtocolState {
    // 未登录
    Unauthenticated,
    // 已登陆
    Authenticated,
}


pub struct Stream<S, D = LengthDelimitedCodec> {
    stream: Option<Framed<S, D>>,
}

impl<S> Stream<S, LengthDelimitedCodec> where
    S: AsyncWrite + AsyncRead + Unpin,

{
    pub fn new(stream: Framed<S, LengthDelimitedCodec>) -> Self {
        Stream {
            stream: Some(stream),
        }
    }

    pub async fn read(&mut self) -> error::Result<Option<RequestMessage>> {
        let frame = self.stream.as_mut().unwrap();
        let buf = Framed::next(frame).await;
        if buf.is_none() {
            return Ok(None);
        }
        let buf = buf.unwrap()?;
        RequestMessage::read(&buf).await
    }
}