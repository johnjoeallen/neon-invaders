use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC_LINK_TTL_SECS: u64 = 15 * 60;
const ACCESS_TOKEN_TTL_SECS: u64 = 15 * 60;
const REFRESH_TOKEN_TTL_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    base_url: String,
    data_path: PathBuf,
    outbox_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Player {
    id: u64,
    email: String,
    display_name: String,
    created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScoreEntry {
    player_id: u64,
    score: u32,
    wave: u32,
    submitted_at: u64,
    app_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshSession {
    player_id: u64,
    token_hash: String,
    issued_at: u64,
    expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedStore {
    next_player_id: u64,
    players: Vec<Player>,
    scores: Vec<ScoreEntry>,
    refresh_sessions: Vec<RefreshSession>,
}

#[derive(Debug, Clone)]
struct MagicLinkSession {
    player_id: u64,
    token_hash: String,
    expires_at: u64,
}

#[derive(Debug, Clone)]
struct AccessSession {
    player_id: u64,
    token_hash: String,
    expires_at: u64,
}

#[derive(Debug, Clone)]
struct Store {
    next_player_id: u64,
    players: Vec<Player>,
    scores: Vec<ScoreEntry>,
    refresh_sessions: Vec<RefreshSession>,
    magic_links: Vec<MagicLinkSession>,
    access_sessions: Vec<AccessSession>,
}

impl Store {
    fn load(path: &PathBuf) -> Self {
        let persisted = fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<PersistedStore>(&text).ok());
        if let Some(persisted) = persisted {
            Self {
                next_player_id: persisted.next_player_id.max(1),
                players: persisted.players,
                scores: persisted.scores,
                refresh_sessions: persisted.refresh_sessions,
                magic_links: Vec::new(),
                access_sessions: Vec::new(),
            }
        } else {
            Self {
                next_player_id: 1,
                players: Vec::new(),
                scores: Vec::new(),
                refresh_sessions: Vec::new(),
                magic_links: Vec::new(),
                access_sessions: Vec::new(),
            }
        }
    }

    fn persist(&self, path: &PathBuf) -> Result<(), ApiError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| ApiError::internal("failed to create data dir"))?;
        }
        let persisted = PersistedStore {
            next_player_id: self.next_player_id,
            players: self.players.clone(),
            scores: self.scores.clone(),
            refresh_sessions: self.refresh_sessions.clone(),
        };
        let text = serde_json::to_string_pretty(&persisted)
            .map_err(|_| ApiError::internal("failed to serialize db"))?;
        fs::write(path, text).map_err(|_| ApiError::internal("failed to write db"))?;
        Ok(())
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.to_string(),
        }
    }

    fn unauthorized(message: &str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.to_string(),
        }
    }

    fn internal(message: &str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct RequestLinkBody {
    email: String,
}

#[derive(Serialize)]
struct RequestLinkResponse {
    ok: bool,
}

#[derive(Deserialize)]
struct VerifyLinkBody {
    token: String,
}

#[derive(Deserialize)]
struct RefreshBody {
    refresh_token: String,
}

#[derive(Deserialize)]
struct SubmitScoreBody {
    score: u32,
    wave: u32,
    app_version: Option<String>,
}

#[derive(Serialize)]
struct AuthResponse {
    player_id: u64,
    email: String,
    display_name: String,
    high_score: u32,
    access_token: String,
    refresh_token: String,
    access_expires_in: u64,
}

#[derive(Serialize)]
struct SubmitScoreResponse {
    accepted: bool,
    personal_best: u32,
    rank: usize,
}

#[derive(Serialize)]
struct LeaderboardEntry {
    rank: usize,
    player_id: u64,
    display_name: String,
    score: u32,
    wave: u32,
}

#[derive(Serialize)]
struct LeaderboardResponse {
    entries: Vec<LeaderboardEntry>,
}

#[derive(Deserialize)]
struct LeaderboardQuery {
    limit: Option<usize>,
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
}

#[tokio::main]
async fn main() {
    let bind = std::env::var("SCORE_SERVER_BIND").unwrap_or_else(|_| "127.0.0.1:8787".to_string());
    let base_url =
        std::env::var("SCORE_SERVER_BASE_URL").unwrap_or_else(|_| format!("http://{bind}"));
    let data_dir = std::env::var("SCORE_SERVER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./score-server/data"));
    let data_path = data_dir.join("db.json");
    let outbox_path = data_dir.join("magic-links.log");

    let state = AppState {
        store: Arc::new(Mutex::new(Store::load(&data_path))),
        base_url,
        data_path,
        outbox_path,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/request-link", post(request_link))
        .route("/auth/verify", post(verify_link))
        .route("/auth/refresh", post(refresh_session))
        .route("/leaderboard", get(leaderboard))
        .route("/scores", post(submit_score))
        .with_state(state);

    let addr: SocketAddr = bind.parse().expect("invalid SCORE_SERVER_BIND");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind score server");
    axum::serve(listener, app)
        .await
        .expect("score server failed");
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

async fn request_link(
    State(state): State<AppState>,
    Json(body): Json<RequestLinkBody>,
) -> Result<Json<RequestLinkResponse>, ApiError> {
    let email = normalize_email(&body.email)?;
    let mut store = state
        .store
        .lock()
        .map_err(|_| ApiError::internal("failed to lock store"))?;
    prune_expired(&mut store);

    let player_id = if let Some(player) = store.players.iter().find(|player| player.email == email)
    {
        player.id
    } else {
        let player_id = store.next_player_id;
        store.next_player_id += 1;
        store.players.push(Player {
            id: player_id,
            email: email.clone(),
            display_name: default_display_name(&email),
            created_at: now_secs(),
        });
        player_id
    };

    let token = generate_token()?;
    let token_hash = hash_token(&token);
    store
        .magic_links
        .retain(|session| session.player_id != player_id);
    store.magic_links.push(MagicLinkSession {
        player_id,
        token_hash,
        expires_at: now_secs() + MAGIC_LINK_TTL_SECS,
    });
    store.persist(&state.data_path)?;

    let link = format!(
        "{}/verify?token={token}",
        state.base_url.trim_end_matches('/')
    );
    write_magic_link(&state.outbox_path, &email, &link)?;
    Ok(Json(RequestLinkResponse { ok: true }))
}

async fn verify_link(
    State(state): State<AppState>,
    Json(body): Json<VerifyLinkBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let token_hash = hash_token(body.token.trim());
    let mut store = state
        .store
        .lock()
        .map_err(|_| ApiError::internal("failed to lock store"))?;
    prune_expired(&mut store);

    let now = now_secs();
    let Some(index) = store
        .magic_links
        .iter()
        .position(|session| session.token_hash == token_hash && session.expires_at >= now)
    else {
        return Err(ApiError::unauthorized("invalid or expired magic link"));
    };
    let session = store.magic_links.remove(index);
    let player = store
        .players
        .iter()
        .find(|player| player.id == session.player_id)
        .cloned()
        .ok_or_else(|| ApiError::internal("player not found"))?;

    let auth = issue_tokens(&mut store, &player)?;
    store.persist(&state.data_path)?;
    Ok(Json(auth))
}

async fn refresh_session(
    State(state): State<AppState>,
    Json(body): Json<RefreshBody>,
) -> Result<Json<AuthResponse>, ApiError> {
    let token_hash = hash_token(body.refresh_token.trim());
    let mut store = state
        .store
        .lock()
        .map_err(|_| ApiError::internal("failed to lock store"))?;
    prune_expired(&mut store);

    let now = now_secs();
    let Some(index) = store
        .refresh_sessions
        .iter()
        .position(|session| session.token_hash == token_hash && session.expires_at >= now)
    else {
        return Err(ApiError::unauthorized("invalid or expired refresh token"));
    };
    let session = store.refresh_sessions.remove(index);
    let player = store
        .players
        .iter()
        .find(|player| player.id == session.player_id)
        .cloned()
        .ok_or_else(|| ApiError::internal("player not found"))?;

    let auth = issue_tokens(&mut store, &player)?;
    store.persist(&state.data_path)?;
    Ok(Json(auth))
}

async fn leaderboard(
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, ApiError> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let store = state
        .store
        .lock()
        .map_err(|_| ApiError::internal("failed to lock store"))?;
    let mut best_by_player: HashMap<u64, ScoreEntry> = HashMap::new();
    for score in &store.scores {
        match best_by_player.get(&score.player_id) {
            Some(existing)
                if existing.score > score.score
                    || (existing.score == score.score && existing.wave >= score.wave) => {}
            _ => {
                best_by_player.insert(score.player_id, score.clone());
            }
        }
    }

    let mut entries: Vec<_> = best_by_player
        .into_iter()
        .filter_map(|(player_id, score)| {
            let player = store.players.iter().find(|player| player.id == player_id)?;
            Some((player.display_name.clone(), player_id, score))
        })
        .collect();
    entries.sort_by(|a, b| {
        b.2.score
            .cmp(&a.2.score)
            .then(b.2.wave.cmp(&a.2.wave))
            .then(a.0.cmp(&b.0))
    });

    let entries = entries
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(i, (display_name, player_id, score))| LeaderboardEntry {
            rank: i + 1,
            player_id,
            display_name,
            score: score.score,
            wave: score.wave,
        })
        .collect();
    Ok(Json(LeaderboardResponse { entries }))
}

async fn submit_score(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SubmitScoreBody>,
) -> Result<Json<SubmitScoreResponse>, ApiError> {
    if body.score == 0 {
        return Err(ApiError::bad_request("score must be greater than zero"));
    }
    let token = bearer_token(&headers)?;
    let token_hash = hash_token(token);
    let mut store = state
        .store
        .lock()
        .map_err(|_| ApiError::internal("failed to lock store"))?;
    prune_expired(&mut store);

    let now = now_secs();
    let Some(session) = store
        .access_sessions
        .iter()
        .find(|session| session.token_hash == token_hash && session.expires_at >= now)
        .cloned()
    else {
        return Err(ApiError::unauthorized("invalid or expired access token"));
    };

    store.scores.push(ScoreEntry {
        player_id: session.player_id,
        score: body.score,
        wave: body.wave,
        submitted_at: now,
        app_version: body.app_version,
    });
    let personal_best = store
        .scores
        .iter()
        .filter(|score| score.player_id == session.player_id)
        .map(|score| score.score)
        .max()
        .unwrap_or(body.score);

    let mut best_scores: Vec<_> = store
        .players
        .iter()
        .filter_map(|player| {
            store
                .scores
                .iter()
                .filter(|score| score.player_id == player.id)
                .max_by(|a, b| a.score.cmp(&b.score).then(a.wave.cmp(&b.wave)))
                .map(|best| (player.id, best.score, best.wave))
        })
        .collect();
    best_scores.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    let rank = best_scores
        .iter()
        .position(|entry| entry.0 == session.player_id)
        .map(|index| index + 1)
        .unwrap_or(best_scores.len());

    store.persist(&state.data_path)?;
    Ok(Json(SubmitScoreResponse {
        accepted: true,
        personal_best,
        rank,
    }))
}

fn issue_tokens(store: &mut Store, player: &Player) -> Result<AuthResponse, ApiError> {
    let now = now_secs();
    let access_token = generate_token()?;
    let refresh_token = generate_token()?;

    store
        .access_sessions
        .retain(|session| session.player_id != player.id);
    store
        .refresh_sessions
        .retain(|session| session.player_id != player.id);
    store.access_sessions.push(AccessSession {
        player_id: player.id,
        token_hash: hash_token(&access_token),
        expires_at: now + ACCESS_TOKEN_TTL_SECS,
    });
    store.refresh_sessions.push(RefreshSession {
        player_id: player.id,
        token_hash: hash_token(&refresh_token),
        issued_at: now,
        expires_at: now + REFRESH_TOKEN_TTL_SECS,
    });

    let high_score = store
        .scores
        .iter()
        .filter(|score| score.player_id == player.id)
        .map(|score| score.score)
        .max()
        .unwrap_or(0);

    Ok(AuthResponse {
        player_id: player.id,
        email: player.email.clone(),
        display_name: player.display_name.clone(),
        high_score,
        access_token,
        refresh_token,
        access_expires_in: ACCESS_TOKEN_TTL_SECS,
    })
}

fn prune_expired(store: &mut Store) {
    let now = now_secs();
    store
        .magic_links
        .retain(|session| session.expires_at >= now);
    store
        .access_sessions
        .retain(|session| session.expires_at >= now);
    store
        .refresh_sessions
        .retain(|session| session.expires_at >= now);
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let Some(header) = headers.get("authorization") else {
        return Err(ApiError::unauthorized("missing authorization header"));
    };
    let value = header
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid authorization header"))?;
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(ApiError::unauthorized("expected bearer token"));
    };
    Ok(token.trim())
}

fn write_magic_link(path: &PathBuf, email: &str, link: &str) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| ApiError::internal("failed to create outbox dir"))?;
    }
    let line = format!("{} {}\n", email, link);
    let mut existing = fs::read_to_string(path).unwrap_or_default();
    existing.push_str(&line);
    fs::write(path, existing)
        .map_err(|_| ApiError::internal("failed to write magic link outbox"))?;
    Ok(())
}

fn normalize_email(input: &str) -> Result<String, ApiError> {
    let email = input.trim().to_ascii_lowercase();
    if email.len() < 5 || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(ApiError::bad_request("invalid email address"));
    }
    Ok(email)
}

fn default_display_name(email: &str) -> String {
    email
        .split('@')
        .next()
        .unwrap_or("pilot")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(16)
        .collect::<String>()
        .if_empty_then("pilot")
}

fn generate_token() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| ApiError::internal("failed to generate token"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

trait StringExt {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}
