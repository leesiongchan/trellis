#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use axum::Router;
use tokio::task::JoinHandle;
pub struct TestServer {
    pub address: std::net::SocketAddr,
    task: JoinHandle<()>,
    data: Option<tempfile::TempDir>,
}
impl TestServer {
    pub fn keep_data_directory(mut self, data: tempfile::TempDir) -> Self {
        self.data = Some(data);
        self
    }
    pub fn origin(&self) -> String {
        format!("http://{}", self.address)
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub async fn spawn_router(router: Router) -> TestServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    TestServer {
        address,
        task,
        data: None,
    }
}
pub fn config(
    origin: &str,
) -> (
    std::sync::Arc<branch_backend::config::Config>,
    tempfile::TempDir,
) {
    let data = tempfile::tempdir().unwrap();
    let config = branch_backend::config::Config::from_values(
        &std::collections::HashMap::from([
            ("APP_ORIGIN".into(), origin.into()),
            (
                "APP_DATA_DIR".into(),
                data.path().join("data").display().to_string(),
            ),
            (
                "APP_MODEL_DIR".into(),
                data.path().join("models").display().to_string(),
            ),
            ("APP_ACCESS_KEY".into(), "a".repeat(64)),
            ("OPENAI_API_KEY".into(), "fake-openai".into()),
            ("TAVILY_API_KEY".into(), "fake-tavily".into()),
        ]),
        &data.path().join("release/branch-backend"),
    )
    .unwrap();
    (std::sync::Arc::new(config), data)
}
