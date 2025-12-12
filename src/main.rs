use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Error;
use axum::{
    Router,
    extract::{Json, State},
    routing::post,
};
use axum::http::StatusCode;
use serde_json::json;
use wreq::{cookie, Proxy, Uri};
use wreq::cookie::{Cookie, CookieStore};
use wreq::redirect::Policy;
use wreq_util::Emulation;
use serde::Deserialize;

#[derive(Clone, Debug)]
struct RunRequest {
    url: String,
    method: String,
    data: String,
    headers: HashMap<String, String>,
    proxy: String,
    timeout: i32,

}
async fn handler(
    Json(payload): Json<RunRequest>,
) -> String {
    let jar = Arc::new(cookie::Jar::default());
    let url = payload.url.clone();
    let data = payload.data.clone().leak();

    let uri = Uri::from_static(url.clone().leak());
    let client = wreq::Client::builder()
        .emulation(Emulation::Chrome142)
        .proxy(Proxy::all(payload.proxy).unwrap())
        .cert_verification(false)
        .cookie_provider(jar.clone())
        .redirect(Policy::default())
        .http1_only()
        .brotli(true)
        // .timeout(Duration::from_secs(20))
        .build().unwrap();
        let resp = match payload.method.to_uppercase().as_str() {
            "GET"=> {
                client.get(url.clone()).send().await.unwrap()
            },
            "POST"=> {
                client.post(url.clone()).body(data.as_bytes()).send().await.unwrap()
            },
            _ =>{
                let json = json!({
                    "code": -1,
                    "message": "方法不支持",
                });
                return serde_json::to_string(&json).unwrap();
            },
        };
    let status = resp.status();
    let resp_headers = resp.headers().clone().into_iter().map(|(k,v)|{
        (k.unwrap().as_str().to_string(), v.to_str().unwrap().to_string())
    }).collect::<HashMap<String, String>>();

    let body = resp.text().await.unwrap();
    let cookies = jar.cookies(&uri).iter().map(|x|{
        let c = x.to_str().unwrap().to_string();
        let i = c.find("=").unwrap();
        (c[0..i].to_string(), c[i+1.. c.len()].to_string())
    }).collect::<HashMap<String, String>>();
    let json = json!({
        "code": 0,
        "text": body,
        "status_code": status.as_u16(),
        "headers": resp_headers,
        "cookies": cookies,
    });


    return serde_json::to_string(&json).unwrap();



}
#[derive(Deserialize)]
struct User {
    name: String,
    email: String,
}
#[tokio::main] // 这里是 MultiThread Runtime，适合 Axum
async fn main() {
    // 启动 Deno Worker
    println!("Deno worker started.");

    // 构建 Axum 路由
    // let app = Router::new()
    //     .route("/forward", post(handler));
    async fn create_user(Json(user): Json<User>) -> String {
        format!("Created user: {} with email {}", user.name, user.email)
    }

    let app = Router::new().route("/users", post(create_user));
    println!("Server running on http://127.0.0.1:3100");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3100").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

