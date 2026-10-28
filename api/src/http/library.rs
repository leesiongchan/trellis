//! Authenticated protocol-2 JSON boundaries. No client-supplied queries or storage paths.
use crate::{
    app::AppState,
    contracts::library::*,
    error::{AppError, ErrorCode},
    http::{guard, headers::context_id},
    knowledge::matching::GraphBounds,
};
use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, de::DeserializeOwned};
use std::sync::Arc;
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PageQuery {
    cursor: Option<String>,
    limit: Option<usize>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogQuery {
    cursor: Option<String>,
    limit: Option<usize>,
    search: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TopicQuery {
    answer_version_id: Option<AnswerVersionId>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocationQuery {
    cursor: Option<String>,
    limit: Option<usize>,
    exploration_id: Option<ExplorationId>,
    answer_version_id: Option<AnswerVersionId>,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphQuery {
    hops: Option<u8>,
    nodes: Option<usize>,
    edges: Option<usize>,
}
fn query<T: DeserializeOwned>(request: &Request) -> Result<T, AppError> {
    Query::<T>::try_from_uri(request.uri())
        .map(|q| q.0)
        .map_err(|_| AppError::invalid_question())
}
async fn command<T: ValidatedInput + DeserializeOwned>(
    request: Request,
    state: &AppState,
) -> Result<Command<T>, AppError> {
    Command::from_json(guard::read_json(request, &state.config.limits).await?)
}
fn same<T: PartialEq>(route: &T, payload: &T) -> Result<(), AppError> {
    if route == payload {
        Ok(())
    } else {
        Err(AppError::new(ErrorCode::NotFound))
    }
}
fn json<T: serde::Serialize>(value: T) -> Response {
    Json(value).into_response()
}
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/library", get(handle))
        .route("/library/purge", post(handle))
        .route("/explorations", get(handle).post(handle))
        .route("/explorations/{id}", get(handle).delete(handle))
        .route("/explorations/{id}/view", get(handle).put(handle))
        .route("/explorations/{id}/appearances", post(handle))
        .route("/appearances/{id}/research", post(handle))
        .route("/appearances/{id}/answer", patch(handle))
        .route("/topics", get(handle))
        .route("/topics/{id}", get(handle))
        .route("/topics/{id}/connections", get(handle))
        .route("/topics/{id}/neighborhood", get(handle))
        .route("/topics/{id}/appearances", get(handle))
        .route("/topics/{id}/explorations", post(handle))
        .route("/connections/{id}", patch(handle))
        .route("/operations/{id}", get(handle))
        .route("/operations", get(handle))
        .route("/imports", post(handle))
        .route("/imports/{id}", get(handle))
        .route("/imports/{id}/batches", post(handle))
        .route("/imports/{id}/completion", post(handle))
}
pub async fn handle(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let id = context_id(&request).to_owned();
    match run(&state, request).await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(event="library_request_failed",requestId=%id,code=error.code());
            error.response(&id)
        }
    }
}
async fn run(state: &Arc<AppState>, request: Request) -> Result<Response, AppError> {
    guard::library_scope(
        state,
        request.headers(),
        request.method() != axum::http::Method::GET,
    )?;
    let path = request.uri().path().to_owned();
    let path = path
        .strip_prefix("/api/")
        .unwrap_or(path.trim_start_matches('/'));
    let parts = path.split('/').collect::<Vec<_>>();
    let method = request.method().clone();
    let store = &state.store;
    Ok(match (method.as_str(), parts.as_slice()) {
        ("GET", ["library"]) => json(store.library_info().await?),
        ("GET", ["explorations"]) => {
            let p: PageQuery = query(&request)?;
            json(
                store
                    .explorations(p.cursor.as_deref(), p.limit.unwrap_or(50))
                    .await?,
            )
        }
        ("POST", ["explorations"]) => json(
            store
                .create_exploration(&command::<CreateExploration>(request, state).await?)
                .await?,
        ),
        ("GET", ["explorations", id]) => json(store.exploration(&ExplorationId::new(*id)?).await?),
        ("GET", ["explorations", id, "view"]) => {
            json(store.exploration_view(&ExplorationId::new(*id)?).await?)
        }
        ("PUT", ["explorations", id, "view"]) => {
            let id = ExplorationId::new(*id)?;
            let mut limits = state.config.limits.clone();
            // At most 2,000 bounded appearance IDs, plus the command envelope.
            limits.body_bytes = 512 * 1024;
            let c = Command::<SaveView>::from_json(guard::read_json(request, &limits).await?)?;
            same(&id, &c.payload.exploration_id)?;
            json(store.save_exploration_view(&c).await?)
        }
        ("DELETE", ["explorations", id]) => {
            let id = ExplorationId::new(*id)?;
            let c = command::<DeleteExploration>(request, state).await?;
            same(&id, &c.payload.exploration_id)?;
            json(store.delete_exploration(&c).await?)
        }
        ("POST", ["explorations", id, "appearances"]) => {
            let id = ExplorationId::new(*id)?;
            let c = command::<AddBranch>(request, state).await?;
            let parent = store.appearance(&c.payload.parent_appearance_id).await?;
            same(&id, &parent.exploration_id)?;
            json(store.add_branch(&c).await?)
        }
        ("POST", ["appearances", id, "research"]) => {
            let id = AppearanceId::new(*id)?;
            let c = command::<ResearchInput>(request, state).await?;
            same(&id, &c.payload.appearance_id)?;
            let op = state.knowledge.start_research(c).await?;
            let pending = op.state == OperationState::Pending;
            let location = format!("/api/operations/{}", op.id);
            let mut response = json(op);
            if pending {
                *response.status_mut() = StatusCode::ACCEPTED;
            }
            response.headers_mut().insert(
                "location",
                HeaderValue::from_str(&location)
                    .map_err(|_| AppError::new(ErrorCode::InternalFailure))?,
            );
            response
        }
        ("PATCH", ["appearances", id, "answer"]) => {
            let id = AppearanceId::new(*id)?;
            let c = command::<ChooseAnswer>(request, state).await?;
            same(&id, &c.payload.appearance_id)?;
            json(store.choose_answer(&c).await?)
        }
        ("GET", ["topics"]) => {
            let p: CatalogQuery = query(&request)?;
            json(
                store
                    .topics(
                        p.cursor.as_deref(),
                        p.limit.unwrap_or(50),
                        p.search.as_deref(),
                    )
                    .await?,
            )
        }
        ("GET", ["topics", id]) => {
            let id = TopicId::new(*id)?;
            let p: TopicQuery = query(&request)?;
            json(store.topic(&id, p.answer_version_id.as_ref()).await?)
        }
        ("GET", ["topics", id, "connections"]) => {
            let id = TopicId::new(*id)?;
            let p: PageQuery = query(&request)?;
            json(
                store
                    .connections(&id, p.cursor.as_deref(), p.limit.unwrap_or(5))
                    .await?,
            )
        }
        ("GET", ["topics", id, "appearances"]) => {
            let id = TopicId::new(*id)?;
            let p: LocationQuery = query(&request)?;
            json(
                store
                    .topic_appearances(
                        &id,
                        p.cursor.as_deref(),
                        p.limit.unwrap_or(5),
                        p.exploration_id.as_ref(),
                        p.answer_version_id.as_ref(),
                    )
                    .await?,
            )
        }
        ("GET", ["topics", id, "neighborhood"]) => {
            let id = TopicId::new(*id)?;
            let p: GraphQuery = query(&request)?;
            let bounds = GraphBounds {
                hops: p.hops.unwrap_or(1),
                nodes: p.nodes.unwrap_or(100),
                edges: p.edges.unwrap_or(200),
            };
            json(
                tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    store.neighborhood(&id, bounds),
                )
                .await
                .map_err(|_| AppError::new(ErrorCode::StorageBusy))??,
            )
        }
        ("POST", ["topics", id, "explorations"]) => {
            let id = TopicId::new(*id)?;
            let c = command::<StartFromTopic>(request, state).await?;
            same(&id, &c.payload.topic_id)?;
            json(store.start_from_topic(&c).await?)
        }
        ("PATCH", ["connections", id]) => {
            let id = ConnectionId::new(*id)?;
            let c = command::<DismissConnection>(request, state).await?;
            same(&id, &c.payload.connection_id)?;
            json(store.dismiss_connection(&c).await?)
        }
        ("GET", ["operations", id]) => json(store.operation(&OperationId::new(*id)?).await?),
        ("GET", ["operations"]) => {
            let p: PageQuery = query(&request)?;
            json(
                store
                    .pending_operations(p.cursor.as_deref(), p.limit.unwrap_or(50))
                    .await?,
            )
        }
        ("POST", ["imports"]) => json(
            store
                .begin_import(&command::<BeginImport>(request, state).await?)
                .await?,
        ),
        ("GET", ["imports", id]) => json(store.import_status(&ImportId::new(*id)?).await?),
        ("POST", ["imports", id, "batches"]) => {
            let id = ImportId::new(*id)?;
            let mut limits = state.config.limits.clone();
            // The existing contract bounds records at 2MiB; allow its bounded command envelope.
            limits.body_bytes = 2 * 1024 * 1024 + 2048;
            let value = guard::read_json(request, &limits).await?;
            if serde_json::to_vec(&value["payload"]["records"])
                .map_err(|_| AppError::invalid_question())?
                .len()
                > 2 * 1024 * 1024
            {
                return Err(AppError::new(ErrorCode::BodyTooLarge));
            }
            let c = Command::<ImportBatch>::from_json(value)?;
            same(&id, &c.payload.import_id)?;
            json(store.stage_import(&c).await?)
        }
        ("POST", ["imports", id, "completion"]) => {
            let id = ImportId::new(*id)?;
            let c = command::<FinalizeImport>(request, state).await?;
            same(&id, &c.payload.import_id)?;
            json(store.finalize_import(&c).await?)
        }
        ("POST", ["library", "purge"]) => json(
            store
                .purge_library(&command::<PurgeLibrary>(request, state).await?)
                .await?,
        ),
        _ => return Err(AppError::new(ErrorCode::NotFound)),
    })
}
