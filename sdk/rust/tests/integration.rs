use pelisearch::{Client, SearchRequest};
use std::collections::HashMap;
use std::env;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

fn test_url() -> String {
    env::var("PELISEARCH_TEST_URL").unwrap_or_else(|_| "http://127.0.0.1:7700".into())
}

async fn setup_client() -> Client {
    let client = Client::from_url(&test_url()).expect("client");
    client.health().await.expect("health");
    client
}

async fn reset_index(client: &Client, name: &str) {
    let _ = client.delete_index(name).await;
    client.create_index(name).await.unwrap();
}

#[tokio::test]
async fn index_management() {
    let client = setup_client().await;
    let index = "sdk_rs_index_mgmt";
    reset_index(&client, index).await;

    let indexes = client.list_indexes().await.unwrap();
    assert!(indexes.iter().any(|n| n == index));

    let info = client.get_index(index).await.unwrap();
    assert_eq!(info.name, index);

    client.delete_index(index).await.unwrap();
}

#[tokio::test]
async fn document_operations() {
    let client = setup_client().await;
    let index = "sdk_rs_docs";
    reset_index(&client, index).await;

    client
        .add_document(
            index,
            "d1",
            HashMap::from([
                ("title".into(), "Mouse".into()),
                ("category".into(), "electronics".into()),
            ]),
        )
        .await
        .unwrap();

    let doc = client.get_document(index, "d1").await.unwrap();
    assert!(doc.contains_key("title") || doc.get("fields").is_some());

    client.delete_document(index, "d1").await.unwrap();
    client.delete_index(index).await.unwrap();
}

#[tokio::test]
async fn search_operations() {
    let client = setup_client().await;
    let index = "sdk_rs_search";
    reset_index(&client, index).await;
    client
        .add_document(
            index,
            "p1",
            HashMap::from([
                ("title".into(), "Wireless Mouse".into()),
                ("category".into(), "electronics".into()),
            ]),
        )
        .await
        .unwrap();

    let results = client
        .search(
            index,
            &SearchRequest {
                q: Some("mouse".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert!(!results.hits.is_empty());
    assert!(results.total > 0);
    for hit in &results.hits {
        assert_eq!(hit.index, index);
        assert!(!hit.document_id.is_empty());
    }

    client.delete_index(index).await.unwrap();
}

#[tokio::test]
async fn recovery_persistence() {
    let data_dir = tempfile::tempdir().unwrap();
    let data_path = data_dir.path().to_str().unwrap();
    let port = pick_port();

    let mut child = start_server(port, data_path);
    let url = format!("http://127.0.0.1:{port}");
    wait_for_health(&url).await;

    let client = Client::from_url(&url).unwrap();
    client.create_index("recipes").await.unwrap();
    client
        .add_document(
            "recipes",
            "r1",
            HashMap::from([("title".into(), "Pancakes".into())]),
        )
        .await
        .unwrap();

    stop_server(&mut child);

    let mut child2 = start_server(port, data_path);
    wait_for_health(&url).await;
    let client2 = Client::from_url(&url).unwrap();

    let indexes = client2.list_indexes().await.unwrap();
    assert!(indexes.contains(&"recipes".into()));

    let results = client2
        .search(
            "recipes",
            &SearchRequest {
                q: Some("pancakes".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(results.hits.len(), 1);

    stop_server(&mut child2);
}

fn pick_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn start_server(port: u16, data_path: &str) -> Child {
    let mut bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    bin.push("../../target/debug/pelisearch-server");
    Command::new(&bin)
        .arg("--port")
        .arg(port.to_string())
        .arg("--data-path")
        .arg(data_path)
        .spawn()
        .expect("start server")
}

async fn wait_for_health(base_url: &str) {
    let client = reqwest::Client::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if client
            .get(format!("{base_url}/health"))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("server not ready");
}

fn stop_server(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}
