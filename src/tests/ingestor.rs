use axum::http::StatusCode;

use crate::tests::TestApp;

#[tokio::test]
async fn ingest_single_entry_returns_200() {
    let app = TestApp::new().await;

    let resp = app
        .post_log("Payments", "info", "Payment of Rs 200 processed")
        .await;
    // println!("{:?}", resp);
    resp.assert_status(StatusCode::OK);

    let json_resp = resp.json::<serde_json::Value>();

    assert_eq!(json_resp["inserted"], 1);
    assert_eq!(json_resp["failed"], 0);
}

#[tokio::test]
async fn ingest_batch_of_multiple_returns_correct_count() {
    let app = TestApp::new().await;

    let resp = app
        .server
        .post("/api/logs")
        .json(&serde_json::json!({
            "logs": [
                { "service": "auth", "level": "info",  "message": "login"  },
                { "service": "auth", "level": "warn",  "message": "retry"  },
                { "service": "auth", "level": "error", "message": "failed" },
            ]
        }))
        .await;

    resp.assert_status(StatusCode::OK);
    assert_eq!(resp.json::<serde_json::Value>()["inserted"], 3);
}

#[tokio::test]
async fn ingest_empty_batch_returns_400() {
    let app = TestApp::new().await;

    let resp = app
        .server
        .post("/api/logs")
        .json(&serde_json::json!({ "logs": [] }))
        .await;
    println!("{:?}", resp);
    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(resp.json::<serde_json::Value>()["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn ingest_with_colon_in_service_name_returns_400() {
    let app = TestApp::new().await;

    let resp = app.post_log("bad:service", "info", "msg").await;

    resp.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(resp.json::<serde_json::Value>()["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn history_returns_ingested_entries_in_order() {
    let mut app = TestApp::new().await;
    app.post_log("billing", "info", "invoice created").await;
    app.post_log("billing", "warn", "payment slow").await;

    let resp = app
        .server
        .get("/api/logs")
        .add_query_param("service", "billing")
        .await;
    println!("{:?}", resp);

    resp.assert_status_ok();
    let json = resp.json::<serde_json::Value>();
    assert_eq!(json["count"], 2);

    let logs = json["logs"].as_array().unwrap();
    assert_eq!(logs[0]["message"], "invoice created");
    assert_eq!(logs[1]["message"], "payment slow");

    let _ = app.cleanup().await;
}

#[tokio::test]
async fn history_unknown_service_returns_404() {
    let app = TestApp::new().await;

    let resp = app
        .server
        .get("/api/logs")
        .add_query_param("service", "dummy-service")
        .await;

    resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(resp.json::<serde_json::Value>()["code"], "NOT_FOUND");
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let app = TestApp::new().await;

    let resp = app.server.get("/path/something").await;

    resp.assert_status(StatusCode::NOT_FOUND);
    assert_eq!(resp.json::<serde_json::Value>()["code"], "NOT_FOUND");
}
