//! Compatibility with existing private access capabilities.
use crate::{
    config::Config,
    contracts::utf16_len,
    error::{AppError, ErrorCode},
};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
pub const ACCESS_COOKIE: &str = "branch_access";
pub fn now_ms() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|t| u64::try_from(t.as_millis()).ok())
        .ok_or_else(|| AppError::new(ErrorCode::InternalFailure))
}
fn mac(key: &str) -> Result<Hmac<Sha256>, AppError> {
    Hmac::<Sha256>::new_from_slice(key.as_bytes())
        .map_err(|_| AppError::new(ErrorCode::InternalFailure))
}
pub fn issue_cookie(key: &str, expires_ms: u64) -> Result<String, AppError> {
    if utf16_len(key) < 32 {
        return Err(AppError::new(ErrorCode::AccessRequired));
    }
    let mut signature = mac(key)?;
    signature.update(format!("branch-session-v1:{expires_ms}").as_bytes());
    Ok(format!(
        "{expires_ms}.{}",
        URL_SAFE_NO_PAD.encode(signature.finalize().into_bytes())
    ))
}
pub fn authorize_cookie(cookie: Option<&str>, key: &str, now_ms: u64) -> bool {
    if utf16_len(key) < 32 {
        return false;
    }
    let Some((time, signature)) = cookie.and_then(|v| v.split_once('.')) else {
        return false;
    };
    let Ok(expires) = time.parse::<u64>() else {
        return false;
    };
    if expires <= now_ms || expires.to_string() != time {
        return false;
    }
    let Ok(bytes) = URL_SAFE_NO_PAD.decode(signature) else {
        return false;
    };
    let Ok(mut hmac) = mac(key) else {
        return false;
    };
    hmac.update(format!("branch-session-v1:{time}").as_bytes());
    hmac.verify_slice(&bytes).is_ok()
}
pub fn cookie_value(headers: &HeaderMap) -> Option<&str> {
    let mut found = None;
    for header in headers.get_all(header::COOKIE) {
        let value = header.to_str().ok()?;
        for part in value.split(';') {
            if let Some((name, value)) = part.trim().split_once('=')
                && name == ACCESS_COOKIE
            {
                if found.is_some() {
                    return None;
                }
                found = Some(value);
            }
        }
    }
    found
}
pub fn routes(config: Arc<Config>) -> Router {
    Router::new()
        .route("/access", get(access))
        .with_state(config)
}
async fn access(State(config): State<Arc<Config>>, uri: Uri) -> Response {
    let key = url::form_urlencoded::parse(uri.query().unwrap_or("").as_bytes())
        .find(|(name, _)| name == "key")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default();
    let matches = utf16_len(config.access_key.expose()) >= 32
        && bool::from(
            Sha256::digest(key.as_bytes())
                .ct_eq(&Sha256::digest(config.access_key.expose().as_bytes())),
        );
    let mut response = if matches {
        match now_ms().and_then(|now| issue_cookie(config.access_key.expose(), now + 604800000)) {
            Ok(token) => {
                let secure = if config.origin.scheme() == "https" {
                    "; Secure"
                } else {
                    ""
                };
                let mut response = StatusCode::SEE_OTHER.into_response();
                response
                    .headers_mut()
                    .insert(header::LOCATION, header::HeaderValue::from_static("/"));
                match header::HeaderValue::from_str(&format!(
                    "{ACCESS_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age=604800{secure}"
                )) {
                    Ok(value) => {
                        response.headers_mut().insert(header::SET_COOKIE, value);
                        response
                    }
                    Err(_) => AppError::new(ErrorCode::InternalFailure).response("access"),
                }
            }
            Err(error) => error.response("access"),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "This private access link is invalid.",
        )
            .into_response()
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "referrer-policy",
        header::HeaderValue::from_static("no-referrer"),
    );
    response
}
