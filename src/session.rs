use std::hash::{Hash, Hasher};
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc::Sender;

use crate::command::Command;

pub trait SessionManager: Send + Sync + 'static {
    type Session: Session;

    fn new_client(&self, client_id: String);

    fn create_session(&self,
                      client_id: String,
                      connect_code: String,
                      sender: Sender<Box<dyn Command>>,
    ) -> Arc<Self::Session>;

    fn remove_session(&self, client_id: &str);

    fn connect_codes(&self) -> Vec<String>;
}

pub trait Session: Send + Sync {
    fn client_id(&self) -> &str;
    fn connect_code(&self) -> &str;
    fn sender(&self) -> Sender<Box<dyn Command>>;
}

#[derive(Default)]
pub struct SessionManagerImpl {
    client_map: DashMap<String, Option<String>>,
    session_map: DashMap<String, Arc<SessionImpl>>,
}

impl SessionManagerImpl {}

impl SessionManager for SessionManagerImpl {
    type Session = SessionImpl;

    fn new_client(&self, client_id: String) {
        self.client_map.insert(client_id, None);
    }

    fn create_session(&self,
                      client_id: String,
                      connect_code: String,
                      sender: Sender<Box<dyn Command>>) -> Arc<Self::Session> {
        let session = {
            Arc::new(SessionImpl {
                client_id,
                connect_code,
                sender,
            })
        };
        if self.client_map.contains_key(&session.client_id) {
            if let Some(v) =
                self.client_map
                    .get(&session.client_id)
                    .and_then(|v| v.clone()) {
                self.session_map.remove(&v);
            }
        }
        self.client_map.insert(session.client_id().into(), Some(session.connect_code().into()));
        self.session_map.insert(session.connect_code().into(), session.clone());
        session
    }


    fn remove_session(&self, client_id: &str) {
        let session = self.session_map.get(client_id);
        if session.is_none() {
            return;
        }
        let session = session.as_deref().unwrap();
        self.client_map.remove(session.client_id());
        self.session_map.remove(session.connect_code());
    }

    fn connect_codes(&self) -> Vec<String> {
        self.session_map.iter()
            .map(|s| s.connect_code().to_string())
            .collect::<Vec<String>>()
    }
}

#[derive(Debug)]
pub struct SessionImpl {
    client_id: String,
    connect_code: String,
    sender: Sender<Box<dyn Command>>,
}

impl PartialEq for SessionImpl {
    fn eq(&self, other: &Self) -> bool {
        self.connect_code == other.connect_code && self.client_id == other.client_id
    }
}

impl Hash for SessionImpl {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.connect_code.hash(state);
        self.client_id.hash(state);
    }
}

impl Session for SessionImpl {
    fn client_id(&self) -> &str {
        &self.client_id
    }
    fn connect_code(&self) -> &str {
        &self.connect_code
    }
    fn sender(&self) -> Sender<Box<dyn Command>> {
        self.sender.clone()
    }
}

impl Drop for SessionImpl {
    fn drop(&mut self) {
        tracing::info!("Session {} dropped", self.connect_code);
    }
}