use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing_subscriber::util::SubscriberInitExt;
use proto_server::{serve, session};
use proto_server::session::SessionManager;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .finish().init();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on({
            let sm = Arc::new(session::SessionManagerImpl::default());
            async {
                let span = tracing::span!(tracing::Level::TRACE, "proto server");
                let _enter = span.enter();

                tokio::spawn({
                    let sm = sm.clone();
                    async move {
                        log_session(sm).await;
                    }
                });
                serve("0.0.0.0:8088", sm).await.unwrap();
            }
        });
}

async fn log_session(sm: Arc<impl SessionManager>){
    loop {
        time::sleep(Duration::from_secs(5)).await;
        let vec = sm.connect_codes();
        tracing::debug!("connect codes: {:?}", vec);
    }
}
