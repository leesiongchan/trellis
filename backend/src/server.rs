//! Bounded HTTP/1 serving on loopback with tracked connection lifetimes.
use crate::{
    config::Limits,
    error::{AppError, ErrorCode},
    jobs::Coordinator,
};
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use std::{future::Future, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::JoinSet,
    time::{Instant, timeout_at},
};
use tokio_util::sync::CancellationToken;
pub struct Server {
    address: SocketAddr,
    listener: TcpListener,
    router: axum::Router,
    jobs: Arc<Coordinator>,
    draining: CancellationToken,
    limits: Limits,
}
impl Server {
    pub async fn bind(
        router: axum::Router,
        bind: SocketAddr,
        jobs: Arc<Coordinator>,
        draining: CancellationToken,
        limits: Limits,
    ) -> Result<Self, AppError> {
        if !bind.ip().is_loopback() || limits.connections == 0 {
            return Err(AppError::new(ErrorCode::InvalidConfig));
        }
        let listener = TcpListener::bind(bind)
            .await
            .map_err(|_| AppError::new(ErrorCode::InvalidConfig))?;
        let address = listener
            .local_addr()
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        Ok(Self {
            address,
            listener,
            router,
            jobs,
            draining,
            limits,
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub async fn run(self, shutdown: impl Future<Output = ()> + Send) -> Result<(), AppError> {
        let Self {
            listener,
            router,
            jobs,
            draining,
            limits,
            ..
        } = self;
        tokio::pin!(shutdown);
        let force = CancellationToken::new();
        let capacity = Arc::new(Semaphore::new(limits.connections));
        let mut connections = JoinSet::new();
        let mut accept_error = None;
        loop {
            tokio::select! {
                biased;
                _=&mut shutdown=>break,
                Some(result)=connections.join_next(),if !connections.is_empty()=>{if result.is_err(){tracing::error!(event="connection_task_failed",entryPoint="runtime",code="INTERNAL_FAILURE");}},
                result=listener.accept()=>match result{
                    Ok((socket,_))=>{
                        if let Ok(permit)=capacity.clone().try_acquire_owned(){
                            let service=router.clone();let drain=draining.clone();let force=force.clone();let headers=limits.headers;
                            connections.spawn(async move{let _permit=permit;connection(socket,service,drain,force,headers).await;});
                        }
                    },
                    Err(_)=>{accept_error=Some(AppError::new(ErrorCode::InternalFailure));break;}
                }
            }
        }
        drop(listener);
        let started = Instant::now();
        jobs.begin_drain();
        draining.cancel();
        tracing::info!(
            event = "shutdown_started",
            entryPoint = "runtime",
            graceSeconds = limits.shutdown_grace.as_secs()
        );
        let drain = async {
            tokio::join!(jobs.idle(), async {
                while connections.join_next().await.is_some() {}
            });
        };
        if timeout_at(started + limits.shutdown_grace, drain)
            .await
            .is_err()
        {
            tracing::warn!(event = "shutdown_cancelling", entryPoint = "runtime");
            jobs.cancel_all();
            force.cancel();
            connections.abort_all();
            timeout_at(started + limits.shutdown_total, async {
                tokio::join!(jobs.idle(), async {
                    while connections.join_next().await.is_some() {}
                });
            })
            .await
            .map_err(|_| AppError::new(ErrorCode::InternalFailure))?;
        }
        tracing::info!(event = "shutdown_completed", entryPoint = "runtime");
        if let Some(error) = accept_error {
            Err(error)
        } else {
            Ok(())
        }
    }
}
async fn connection(
    socket: TcpStream,
    router: axum::Router,
    draining: CancellationToken,
    force: CancellationToken,
    headers: Duration,
) {
    let mut stream = tokio_io_timeout::TimeoutStream::new(socket);
    stream.set_write_timeout(Some(Duration::from_secs(15)));
    let io = TokioIo::new(Box::pin(stream));
    let service = TowerToHyperService::new(router);
    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .header_read_timeout(headers)
        .max_headers(64)
        .max_header_size(32768)
        .max_buf_size(65536)
        .half_close(false)
        .keep_alive(true);
    let connection = builder.serve_connection(io, service);
    tokio::pin!(connection);
    let mut closing = false;
    loop {
        tokio::select! {
            biased;
            _=force.cancelled()=>break,
            _=draining.cancelled(),if !closing=>{connection.as_mut().graceful_shutdown();closing=true;},
            result=&mut connection=>{if result.is_err(){tracing::debug!(event="connection_closed",entryPoint="http",code="PROTOCOL_OR_IO");}break;}
        }
    }
}
