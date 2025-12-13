use anyhow::Error;
use axum::http::StatusCode;
use axum::{
    Router,
    extract::{Json, State},
    routing::post,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use wreq::cookie::{Cookie, CookieStore};
use wreq::redirect::Policy;
use wreq::{Proxy, Uri, cookie, head, header};
use wreq_util::Emulation;

#[derive(Clone, Debug, Deserialize)]
struct RunRequest {
    url: String,
    method: String,
    data: String,
    headers: HashMap<String, String>,
    proxy: String,
    timeout: i32,
}

async fn forward(payload: RunRequest) -> Result<serde_json::Value, anyhow::Error> {
    let jar = Arc::new(cookie::Jar::default());
    let url = payload.url.clone();
    let data = payload.data.clone().leak();

    let uri = Uri::from_static(url.clone().leak());
    let hs = payload.headers.clone();
    let client = wreq::Client::builder()
        .emulation(Emulation::Chrome141)
        .proxy(Proxy::all(payload.proxy).unwrap())
        .cert_verification(false)
        .cookie_provider(jar.clone())
        .redirect(Policy::default())
        .brotli(true)
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut headers = header::HeaderMap::new();
    hs.into_iter().for_each(|(k, v)| {
        headers.insert(
            header::HeaderName::from_static(k.to_lowercase().leak()),
            header::HeaderValue::from_str(v.as_str()).unwrap(),
        );
    });
    println!("headers {:?}", headers);
    let resp = match payload.method.to_uppercase().as_str() {
        "GET" => client
            .get(url.clone())
            .headers(headers)
            .send()
            .await?,
        "POST" => client
            .post(url.clone())
            .headers(headers)
            .body(data.as_bytes())
            .send()
            .await?,
        _ => {
            let json = json!({
                "code": -1,
                "message": "方法不支持",
            });
            return Ok(json);
        }
    };
    let status = resp.status();
    let resp_headers = resp
        .headers()
        .clone()
        .into_iter()
        .filter_map(|(k, v)| {
            if k.is_none() {
                return None;
            } else {
                return Some(
                    (k.unwrap().as_str().to_string(),
                     v.to_str().unwrap().to_string(),)
                );
            }
        })
        .collect::<HashMap<String, String>>();

    let body = resp.text().await?;
    let cookies = jar
        .cookies(&uri)
        .iter()
        .map(|x| {
            let c = x.to_str().unwrap().to_string();
            let i = c.find("=").unwrap();
            (c[0..i].to_string(), c[i + 1..c.len()].to_string())
        })
        .collect::<HashMap<String, String>>();
    Ok(json!({
        "code": 0,
        "text": body,
        "status_code": status.as_u16(),
        "headers": resp_headers,
        "cookies": cookies,
    }))
}
async fn handler(Json(payload): Json<RunRequest>) -> String {
    let t1 = Instant::now();
    match forward(payload).await{

        Ok(mut res) => {
            res["cost"] = (t1.elapsed().as_millis()as u64).into();
            serde_json::to_string(&res).unwrap()
        },
        Err(err) => {
            serde_json::to_string(&json!({"code":400,"message": err.to_string(),"cost": (t1.elapsed().as_millis()as u64)}).to_string()).unwrap()
        }
    }


}
#[derive(Deserialize)]
struct User {
    name: String,
    email: String,
}
async fn create_user(Json(user): Json<User>) -> String {
    format!("Created user: {} with email {}", user.name, user.email)
}
#[tokio::main] // 这里是 MultiThread Runtime，适合 Axum
async fn main() {
    // 启动 Deno Worker
    println!("Deno worker started.");

    // 构建 Axum 路由
    // let app = Router::new()
    //     .route("/forward", post(handler));

    let app = Router::new().route("/akamaisrv/forward", post(handler));
    println!("Server running on http://127.0.0.1:3100");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3100").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
