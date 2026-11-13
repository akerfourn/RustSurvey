use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use rustsurvey_migration::{Migrator, MigratorTrait, SchemaManager};
use rustsurvey_server::workspace::{
    AppError, PasswordPolicy, Runtime, WorkspaceState, bootstrap_admin, routes,
};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    sea_query::{Expr, ExprTrait, Query},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc,
    atomic::{AtomicI64, AtomicU64, Ordering},
};
use tower::ServiceExt;

const PASSWORD: &str = "synthetic-admin-password";
#[path = "workspace/exports.rs"]
mod export_tests;
#[path = "workspace/history.rs"]
mod history_tests;
#[path = "workspace/local_storage.rs"]
mod local_storage_tests;
#[path = "workspace/media.rs"]
mod media_tests;
#[path = "../../object-store/tests/common/mod.rs"]
mod object_contract;
#[path = "../../object-store/tests/common/directory.rs"]
mod test_directory;

#[tokio::test]
async fn test_sql_objects_should_respect_the_same_contract_as_the_memory_adapter() {
    let fixture = fixture().await;
    object_contract::contract(&rustsurvey_server::object_store::SqlObjectStore::new(
        fixture.database,
    ))
    .await;
}
struct FixedRuntime {
    now: AtomicI64,
    serial: AtomicU64,
}
impl FixedRuntime {
    fn new() -> Self {
        Self {
            now: AtomicI64::new(1_800_000_000),
            serial: AtomicU64::new(1),
        }
    }
}
impl Runtime for FixedRuntime {
    fn now(&self) -> Result<i64, AppError> {
        Ok(self.now.load(Ordering::SeqCst))
    }
    fn random(&self) -> Result<[u8; 32], AppError> {
        Ok(Sha256::digest(self.serial.fetch_add(1, Ordering::SeqCst).to_be_bytes()).into())
    }
}
struct Fixture {
    app: Router,
    database: DatabaseConnection,
    runtime: Arc<FixedRuntime>,
}
async fn fixture() -> Fixture {
    let options = rustsurvey_server::database::options("sqlite::memory:".to_owned());
    let database = Database::connect(options).await.unwrap();
    initialize(database).await
}
async fn initialize(database: DatabaseConnection) -> Fixture {
    Migrator::up(&database, None).await.unwrap();
    Migrator::up(&database, None).await.unwrap();
    let runtime = Arc::new(FixedRuntime::new());
    let state = WorkspaceState::with_runtime(database.clone(), false, runtime.clone());
    assert!(
        bootstrap_admin(&state, "admin", PASSWORD, PasswordPolicy::default())
            .await
            .unwrap()
    );
    Fixture {
        app: routes(state),
        database,
        runtime,
    }
}

async fn unconfigured_workspace() -> (DatabaseConnection, WorkspaceState) {
    let options = rustsurvey_server::database::options("sqlite::memory:".to_owned());
    let database = Database::connect(options).await.unwrap();
    Migrator::up(&database, None).await.unwrap();
    let state =
        WorkspaceState::with_runtime(database.clone(), false, Arc::new(FixedRuntime::new()));
    (database, state)
}

#[tokio::test]
async fn test_bootstrap_should_explain_invalid_usernames_and_reject_short_passwords_by_default() {
    let (_, state) = unconfigured_workspace().await;
    for (username, password, code, message) in [
        (
            "admin@example",
            PASSWORD,
            "invalid_administrator_username",
            "lettres ASCII",
        ),
        (
            "admin",
            "dev",
            "invalid_administrator_password",
            "12 caractères",
        ),
        (
            "admin",
            "éééééé",
            "invalid_administrator_password",
            "12 caractères",
        ),
        ("admin", "", "invalid_administrator_password", "vide"),
    ] {
        let error = bootstrap_admin(&state, username, password, PasswordPolicy::default())
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, code);
        assert!(
            error.message.contains(message),
            "unexpected bootstrap diagnostic: {}",
            error.message
        );
    }
    assert!(
        bootstrap_admin(&state, "admin", PASSWORD, PasswordPolicy::default())
            .await
            .unwrap()
    );
}

#[cfg(debug_assertions)]
#[tokio::test]
async fn test_development_bootstrap_should_allow_short_passwords_with_normal_hashed_authentication()
{
    let (database, state) = unconfigured_workspace().await;
    let policy = PasswordPolicy::from_development_flag(true).unwrap();
    for (password, message) in [(String::new(), "vide"), ("x".repeat(1025), "1024 octets")] {
        let error = bootstrap_admin(&state, "admin", &password, policy)
            .await
            .unwrap_err();
        assert_eq!(error.code, "invalid_administrator_password");
        assert!(error.message.contains(message));
    }
    assert!(
        bootstrap_admin(&state, "admin", "dev", policy)
            .await
            .unwrap()
    );
    assert!(
        !bootstrap_admin(&state, "admin", PASSWORD, PasswordPolicy::default())
            .await
            .unwrap()
    );
    let account = database
        .query_one(
            &Query::select()
                .column("password_hash")
                .from("administrator")
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    let hash: String = account.try_get("", "password_hash").unwrap();
    assert!(hash.starts_with("$argon2id$"));
    let app = routes(state);
    let response = call(
        &app,
        "POST",
        "/api/admin/login",
        Some(json!({"username":"admin","password":"dev"})),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await["authenticated"], true);
}

fn definition() -> Value {
    let legacy = json!({"schemaVersion":1,"title":"Questionnaire enregistré","language":"fr","welcome":"Bienvenue","end":"Merci","groups":[{"id":"g1","title":"Premier groupe","description":"","questions":[{"id":"q1","code":"NAME","title":"Un nom de projet","help":"","kind":"short_text","mandatory":"strict","choices":[]}]}]});
    let mut definition: rustsurvey_domain::Definition = serde_json::from_value(legacy).unwrap();
    definition.upgrade().unwrap();
    serde_json::to_value(definition).unwrap()
}
async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    cookie: Option<&str>,
    csrf: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("x-requested-with", "RustSurvey");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    let payload = if let Some(body) = body {
        request = request.header(header::CONTENT_TYPE, "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(payload).unwrap())
        .await
        .unwrap()
}
async fn json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap()
}
async fn login(fixture: &Fixture) -> (String, String) {
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/login",
        Some(json!({"username":"admin","password":PASSWORD})),
        None,
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = json(response).await;
    (cookie, body["csrfToken"].as_str().unwrap().to_owned())
}
async fn create(fixture: &Fixture, cookie: &str, csrf: &str, definition: Value) -> Value {
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/studies",
        Some(definition),
        Some(cookie),
        Some(csrf),
    )
    .await;
    let status = response.status();
    let body = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body
}
#[tokio::test]
async fn test_administration_should_require_authentication_and_csrf_before_writing() {
    let fixture = fixture().await;
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(definition()),
            None,
            None
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let (cookie, csrf) = login(&fixture).await;
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(definition()),
            Some(&cookie),
            None
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(definition()),
            Some(&cookie),
            Some("wrong")
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    let path = format!("/api/studies/{}", stored["id"].as_str().unwrap());
    let response = call(&fixture.app, "GET", &path, None, None, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(json(response).await["definition"], definition());
    assert!(
        !SchemaManager::new(&fixture.database)
            .has_table("responses")
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn test_revisions_should_preserve_history_and_reject_stale_editor_writes() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    let id = stored["id"].as_str().unwrap();
    let path = format!("/api/admin/studies/{id}");
    let mut updated = definition();
    updated["title"] = json!("Version deux");
    let response = call(
        &fixture.app,
        "PUT",
        &path,
        Some(json!({"expectedRevision":1,"definition":updated})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(json(response).await["revision"], 2);
    let response = call(
        &fixture.app,
        "PUT",
        &path,
        Some(json!({"expectedRevision":1,"definition":definition()})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let rows = fixture
        .database
        .query_all(
            &Query::select()
                .columns(["revision", "document"])
                .from("definition_revisions")
                .and_where(Expr::col("study_id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let initial: String = rows
        .iter()
        .find(|row| row.try_get::<i64>("", "revision").unwrap() == 1)
        .unwrap()
        .try_get("", "document")
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&initial).unwrap(),
        definition()
    );
    assert_eq!(
        json(
            call(
                &fixture.app,
                "GET",
                &format!("/api/studies/{id}"),
                None,
                None,
                None
            )
            .await
        )
        .await["definition"]["title"],
        "Version deux"
    );
}
#[tokio::test]
async fn test_sessions_should_survive_router_restart_and_be_revoked_on_logout_or_expiry() {
    let mut fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let state =
        WorkspaceState::with_runtime(fixture.database.clone(), true, fixture.runtime.clone());
    assert!(
        !bootstrap_admin(
            &state,
            "changed",
            "another-synthetic-password",
            PasswordPolicy::default()
        )
        .await
        .unwrap()
    );
    fixture.app = routes(state);
    assert_eq!(
        json(
            call(
                &fixture.app,
                "GET",
                "/api/admin/session",
                None,
                Some(&cookie),
                None
            )
            .await
        )
        .await["authenticated"],
        true
    );
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/logout",
        None,
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let header = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(
        header.contains("HttpOnly")
            && header.contains("SameSite=Strict")
            && header.contains("Secure")
            && header.contains("Max-Age=0")
    );
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(definition()),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let (cookie, csrf) = login(&fixture).await;
    fixture
        .runtime
        .now
        .fetch_add(12 * 60 * 60 + 1, Ordering::SeqCst);
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(definition()),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        json(
            call(
                &fixture.app,
                "GET",
                "/api/admin/session",
                None,
                Some(&cookie),
                None
            )
            .await
        )
        .await["authenticated"],
        false
    );
}
#[tokio::test]
async fn test_passwords_and_sessions_should_be_hashed_and_failed_logins_throttled() {
    let fixture = fixture().await;
    let (cookie, _) = login(&fixture).await;
    let account = fixture
        .database
        .query_one(
            &Query::select()
                .column("password_hash")
                .from("administrator")
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    let hash: String = account.try_get("", "password_hash").unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert!(!hash.contains(PASSWORD));
    let session = fixture
        .database
        .query_one(
            &Query::select()
                .column("token_hash")
                .from("administrator_sessions")
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        session.try_get::<String>("", "token_hash").unwrap(),
        cookie.split_once('=').unwrap().1
    );
    for _ in 0..5 {
        assert_eq!(
            call(
                &fixture.app,
                "POST",
                "/api/admin/login",
                Some(json!({"username":"admin","password":"wrong"})),
                None,
                None
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/login",
            Some(json!({"username":"admin","password":PASSWORD})),
            None,
            None
        )
        .await
        .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    fixture.runtime.now.fetch_add(61, Ordering::SeqCst);
    let _ = login(&fixture).await;
}
#[tokio::test]
async fn test_invalid_definitions_and_oversized_bodies_should_not_create_records() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    let mut invalid = definition();
    invalid["schemaVersion"] = json!(4);
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/studies",
        Some(invalid),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        json(response).await["error"]["issues"][0]["path"],
        "schemaVersion"
    );
    let mut invalid = definition();
    invalid["end"] = json!("x".repeat(rustsurvey_domain::MAX_DOCUMENT_BYTES + 1));
    assert_eq!(
        call(
            &fixture.app,
            "POST",
            "/api/admin/studies",
            Some(invalid),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(
        json(call(&fixture.app, "GET", "/api/studies", None, None, None).await).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

async fn large_document_roundtrip(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let mut document = definition();
    document["title"] = json!("Unicode é ' ; DROP TABLE studies; --");
    document["groups"][0]["questions"] = json!((0..12).map(|index| json!({"id":format!("q{index}"),"code":format!("Q{index}"),"title":"Question","help":"é".repeat(9000),"kind":"short_text","mandatory":"no","choices":[]})).collect::<Vec<_>>());
    let document = serde_json::to_value(
        serde_json::from_value::<rustsurvey_domain::Definition>(document).unwrap(),
    )
    .unwrap();
    assert!(document.to_string().len() > 65536);
    let stored = create(fixture, &cookie, &csrf, document.clone()).await;
    let public = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/studies/{}", stored["id"].as_str().unwrap()),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(public["definition"], document);
}

async fn concurrent_updates(fixture: &Fixture) {
    let (cookie, csrf) = login(fixture).await;
    let stored = create(fixture, &cookie, &csrf, definition()).await;
    let id = stored["id"].as_str().unwrap();
    let path = format!("/api/admin/studies/{id}");
    let mut first = definition();
    first["title"] = json!("First editor");
    let mut second = definition();
    second["title"] = json!("Second editor");
    let (first_response, second_response) = tokio::join!(
        call(
            &fixture.app,
            "PUT",
            &path,
            Some(json!({"expectedRevision":1,"definition":first})),
            Some(&cookie),
            Some(&csrf)
        ),
        call(
            &fixture.app,
            "PUT",
            &path,
            Some(json!({"expectedRevision":1,"definition":second})),
            Some(&cookie),
            Some(&csrf)
        )
    );
    let statuses = [first_response.status(), second_response.status()];
    assert!(statuses.contains(&StatusCode::OK), "{statuses:?}");
    assert!(statuses.contains(&StatusCode::CONFLICT), "{statuses:?}");
    let winning_response = if first_response.status() == StatusCode::OK {
        first_response
    } else {
        second_response
    };
    let saved = json(winning_response).await;
    let public = json(
        call(
            &fixture.app,
            "GET",
            &format!("/api/studies/{id}"),
            None,
            None,
            None,
        )
        .await,
    )
    .await;
    assert_eq!(saved["revision"], 2);
    assert_eq!(public, saved);
    let revisions = fixture
        .database
        .query_all(
            &Query::select()
                .column("revision")
                .from("definition_revisions")
                .and_where(Expr::col("study_id").eq(id))
                .to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(
        revisions.len(),
        2,
        "a conflicting write must not add a revision"
    );
}

fn reference_lss() -> Vec<u8> {
    let original = include_str!("../../../examples/surveys/controls.lss");
    let position = original.rfind("</document>").unwrap();
    let mut xml = original.to_owned();
    xml.insert_str(
        position,
        &format!("<!--SOURCE_ONLY_ORIGINAL{}-->", "x".repeat(70000)),
    );
    xml.into_bytes()
}

async fn import_request(
    fixture: &Fixture,
    xml: Vec<u8>,
    cookie: Option<&str>,
    csrf: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/admin/imports/lss")
        .header(header::CONTENT_TYPE, "application/xml");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    fixture
        .app
        .clone()
        .oneshot(request.body(Body::from(xml)).unwrap())
        .await
        .unwrap()
}

async fn import_roundtrip(fixture: &Fixture) {
    let xml = reference_lss();
    assert_eq!(
        import_request(fixture, xml.clone(), None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let (cookie, csrf) = login(fixture).await;
    assert_eq!(
        import_request(fixture, xml.clone(), Some(&cookie), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = import_request(fixture, xml.clone(), Some(&cookie), Some(&csrf)).await;
    let status = response.status();
    let stored = json(response).await;
    assert_eq!(status, StatusCode::CREATED, "{stored}");
    assert_eq!(stored["definition"]["schemaVersion"], 3);
    assert_eq!(stored["definition"]["languages"], json!(["fr", "en"]));
    let id = stored["id"].as_str().unwrap();
    assert!(
        !stored.to_string().contains("SOURCE_ONLY_ORIGINAL"),
        "the original XML must stay private"
    );
    let all_questions: Vec<_> = stored["definition"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|group| group["questions"].as_array().unwrap())
        .collect();
    assert_eq!(all_questions.len(), 28);
    assert!(
        all_questions
            .iter()
            .any(|question| question["kind"] == "dual_scale")
    );
    assert!(
        all_questions
            .iter()
            .any(|question| question["kind"] == "equation")
    );
    assert!(
        all_questions
            .iter()
            .any(|question| !question["subquestions"].as_array().unwrap().is_empty())
    );
    let original_path = format!("/api/admin/studies/{id}/original.lss");
    assert_eq!(
        call(&fixture.app, "GET", &original_path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = call(
        &fixture.app,
        "GET",
        &original_path,
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap()
            .as_ref(),
        xml
    );
    let public_path = format!("/api/studies/{id}");
    let public = json(call(&fixture.app, "GET", &public_path, None, None, None).await).await;
    assert_eq!(public, stored);
    let mut invalid = stored["definition"].clone();
    invalid["title"] = json!("Forbidden content edit");
    let update_path = format!("/api/admin/studies/{id}");
    assert_eq!(
        call(
            &fixture.app,
            "PUT",
            &update_path,
            Some(json!({"expectedRevision":1,"definition":invalid})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut edited = stored["definition"].clone();
    edited["groups"][2]["questions"][0]["relevance"] = json!("MISSING_VARIABLE == 'Y'");
    assert_eq!(
        call(
            &fixture.app,
            "PUT",
            &update_path,
            Some(json!({"expectedRevision":1,"definition":edited})),
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let first_code = all_questions[0]["code"].as_str().unwrap();
    edited["groups"][2]["questions"][0]["relevance"] = json!(format!("{first_code} == 'visible'"));
    edited["groups"][2]["relevance"] = json!(format!("!is_empty({first_code})"));
    let response = call(
        &fixture.app,
        "PUT",
        &update_path,
        Some(json!({"expectedRevision":1,"definition":edited})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    let status = response.status();
    let saved = json(response).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["revision"], 2);
    assert_eq!(saved["definition"], edited);
    let modified_path = format!("/api/admin/studies/{id}/revisions/2/modified.lss");
    assert_eq!(
        call(&fixture.app, "GET", &modified_path, None, None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let modified = call(
        &fixture.app,
        "GET",
        &modified_path,
        None,
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(modified.status(), StatusCode::OK);
    let modified = to_bytes(modified.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let parsed = rustsurvey_limesurvey::LssDocument::parse(
        &modified,
        rustsurvey_limesurvey::Limits::default(),
    )
    .unwrap();
    let projected = parsed.source_model().unwrap();
    let group_id: u64 = saved["definition"]["groups"][2]["source"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let question_id: u64 = saved["definition"]["groups"][2]["questions"][0]["source"]["id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        projected
            .groups
            .iter()
            .find(|group| group.id.0 == group_id)
            .unwrap()
            .relevance,
        format!("!is_empty({first_code})")
    );
    assert_eq!(
        projected
            .questions
            .iter()
            .find(|question| question.id.0 == question_id)
            .unwrap()
            .relevance,
        format!("{first_code} == 'visible'")
    );
    assert!(
        std::str::from_utf8(&modified)
            .unwrap()
            .contains("SOURCE_ONLY_ORIGINAL")
    );
    let state =
        WorkspaceState::with_runtime(fixture.database.clone(), false, fixture.runtime.clone());
    let restarted = routes(state);
    assert_eq!(
        json(call(&restarted, "GET", &public_path, None, None, None).await).await,
        saved
    );
    let response = call(&restarted, "GET", &original_path, None, Some(&cookie), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap()
            .as_ref(),
        xml
    );
}

#[tokio::test]
async fn test_import_should_preserve_all_types_original_xml_and_reopen_edited_conditions() {
    import_roundtrip(&fixture().await).await;
}

#[tokio::test]
async fn test_invalid_lss_and_cycles_should_not_create_partial_records_or_revisions() {
    let fixture = fixture().await;
    let (cookie, csrf) = login(&fixture).await;
    assert_eq!(
        import_request(&fixture, b"<broken>".to_vec(), Some(&cookie), Some(&csrf))
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        import_request(
            &fixture,
            vec![b'x'; 8 * 1024 * 1024 + 1],
            Some(&cookie),
            Some(&csrf)
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let mut definition = definition();
    definition["groups"][0]["relevance"] = json!("!is_empty(NAME)");
    let response = call(
        &fixture.app,
        "POST",
        "/api/admin/studies",
        Some(definition),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        json(response).await["error"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["message"].as_str().unwrap().contains("cycle"))
    );
    assert!(
        json(call(&fixture.app, "GET", "/api/studies", None, None, None).await).await["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn test_second_migration_should_preserve_v1_documents_and_upgrade_them_on_read() {
    let database = Database::connect(rustsurvey_server::database::options(
        "sqlite::memory:".to_owned(),
    ))
    .await
    .unwrap();
    Migrator::up(&database, Some(1)).await.unwrap();
    let legacy=json!({"schemaVersion":1,"title":"Legacy","language":"fr","welcome":"","end":"","groups":[]}).to_string();
    let id = "00000000-0000-4000-8000-000000000001";
    database
        .execute(
            Query::insert()
                .into_table("studies")
                .columns(["id", "title", "revision", "created_at", "updated_at"])
                .values([
                    id.into(),
                    "Legacy".into(),
                    1_i64.into(),
                    0_i64.into(),
                    0_i64.into(),
                ])
                .unwrap(),
        )
        .await
        .unwrap();
    database
        .execute(
            Query::insert()
                .into_table("definition_revisions")
                .columns(["study_id", "revision", "document", "created_at"])
                .values([id.into(), 1_i64.into(), legacy.clone().into(), 0_i64.into()])
                .unwrap(),
        )
        .await
        .unwrap();
    Migrator::up(&database, None).await.unwrap();
    Migrator::up(&database, None).await.unwrap();
    let app = routes(WorkspaceState::with_runtime(
        database.clone(),
        false,
        std::sync::Arc::new(FixedRuntime::new()),
    ));
    let stored =
        json(call(&app, "GET", &format!("/api/studies/{id}"), None, None, None).await).await;
    assert_eq!(stored["definition"]["schemaVersion"], 3);
    assert_eq!(stored["definition"]["title"], "Legacy");
    assert_eq!(stored["revision"], 1);
    let row = database
        .query_one(
            &Query::select()
                .column("document")
                .from("definition_revisions")
                .to_owned(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<String>("", "document").unwrap(),
        legacy,
        "historical revisions must not be rewritten"
    );
}

#[tokio::test]
async fn test_concurrent_updates_should_commit_exactly_one_revision() {
    concurrent_updates(&fixture().await).await;
}

#[tokio::test]
async fn test_login_should_reject_missing_client_header_and_ambiguous_session_cookies() {
    let fixture = fixture().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/admin/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username":"admin","password":PASSWORD}).to_string(),
        ))
        .unwrap();
    let response = fixture.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let (cookie, _) = login(&fixture).await;
    let duplicate = format!("{cookie}; {cookie}");
    let response = call(
        &fixture.app,
        "GET",
        "/api/admin/session",
        None,
        Some(&duplicate),
        None,
    )
    .await;
    assert_eq!(json(response).await["authenticated"], false);
}
#[tokio::test]
async fn test_sqlite_should_roundtrip_large_unicode_documents_and_create_schema_only_once() {
    large_document_roundtrip(&fixture().await).await;
}

#[tokio::test]
#[ignore = "requires a fresh disposable PostgreSQL or MariaDB database"]
async fn test_external_database_should_apply_migrations_and_roundtrip_large_documents() {
    let url = std::env::var("RUSTSURVEY_TEST_DATABASE_URL")
        .expect("provide a fresh local rustsurvey_test database");
    assert!(
        url.contains("127.0.0.1") && url.ends_with("/rustsurvey_test"),
        "only a disposable local database is allowed"
    );
    let mut options = ConnectOptions::new(url);
    options.max_connections(2).sqlx_logging(false);
    let database = Database::connect(options).await.unwrap();
    assert!(
        !SchemaManager::new(&database)
            .has_table("studies")
            .await
            .unwrap(),
        "the database must be fresh; no existing schema will be removed"
    );
    let fixture = initialize(database).await;
    large_document_roundtrip(&fixture).await;
    import_roundtrip(&fixture).await;
    concurrent_updates(&fixture).await;
    history_tests::roundtrip(&fixture).await;
    media_tests::roundtrip(&fixture).await;
    object_contract::contract(&rustsurvey_server::object_store::SqlObjectStore::new(
        fixture.database.clone(),
    ))
    .await;
    let (cookie, csrf) = login(&fixture).await;
    let stored = create(&fixture, &cookie, &csrf, definition()).await;
    let response = call(
        &fixture.app,
        "PUT",
        &format!("/api/admin/studies/{}", stored["id"].as_str().unwrap()),
        Some(json!({"expectedRevision":1,"definition":definition()})),
        Some(&cookie),
        Some(&csrf),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
}
