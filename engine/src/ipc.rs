//! HTTP API 服务器（127.0.0.1:17320）
//!
//! 为 Tauri 工具面板提供 REST 接口，通过 JSON 交换设置/日志/状态。
//! 纯 std::net 手撸，无 tokio 依赖，单线程 per-connection 模型。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::log::LogStorage;
use crate::types::{PanelStatus, RadarData, Settings};

/// 启动 HTTP API 服务器（在后台线程运行）
pub fn start_server(
    port: u16,
    settings: Arc<Mutex<Settings>>,
    logs: Arc<Mutex<LogStorage>>,
    radar_data: Arc<Mutex<RadarData>>,
) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("IPC server: failed to bind port");

    println!("[IPC] API server listening on http://{}", addr);

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let (s, l, r) = (settings.clone(), logs.clone(), radar_data.clone());
                thread::spawn(move || handle(stream, &s, &l, &r));
            }
        }
    });
}

// ── 请求解析 ────────────────────────────────────────────────────

struct Request {
    method: String,
    path: String,
    query: String,
    body: String,
}

fn parse_request(raw: &str) -> Option<Request> {
    let mut lines = raw.lines();
    let request_line = lines.next()?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    let method = parts[0].to_uppercase();
    let full_path = parts[1];
    let (path, query) = match full_path.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (full_path.to_string(), String::new()),
    };

    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.trim().to_string())
        .unwrap_or_default();

    Some(Request {
        method,
        path,
        query,
        body,
    })
}

fn query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? == name {
            return Some(parts.next().unwrap_or(""));
        }
    }
    None
}

// ── 路由 ────────────────────────────────────────────────────────

fn handle(
    mut stream: TcpStream,
    settings: &Arc<Mutex<Settings>>,
    logs: &Arc<Mutex<LogStorage>>,
    radar_data: &Arc<Mutex<RadarData>>,
) {
    // 先读头部（4KB 足够）
    let mut buf = vec![0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };

    // 解析 Content-Length，按需读取完整请求体
    let raw_str = String::from_utf8_lossy(&buf[..n]);
    let content_len = raw_str
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split(':').nth(1)?.trim().parse::<usize>().ok());

    let raw: String = if let Some(len) = content_len {
        let header_end = raw_str.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
        let total_needed = header_end + len;
        if total_needed > buf.len() {
            buf.resize(total_needed.min(524_288), 0); // 上限 512KB
        }
        let mut total = n;
        while total < total_needed.min(buf.len()) {
            match stream.read(&mut buf[total..]) {
                Ok(0) => break,
                Ok(r) => total += r,
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&buf[..total]).to_string()
    } else {
        raw_str.to_string()
    };

    let req = match parse_request(&raw) {
        Some(r) => r,
        None => return send_cors(&mut stream, 400, "Bad Request"),
    };

    if req.method == "OPTIONS" {
        return send_cors(&mut stream, 204, "");
    }

    // 二进制资源（logo）走独立路径
    if req.path == "/api/resources/logo" && req.method == "GET" {
        let png = include_bytes!("../resources/title-zh-en.png");
        return send_binary(&mut stream, 200, "image/png", png);
    }

    match route(&req, settings, logs, radar_data) {
        Some((status, body)) => send_cors(&mut stream, status, &body),
        None => send_cors(&mut stream, 404, r#"{"error":"Not Found"}"#),
    }
}

fn route(
    req: &Request,
    settings: &Arc<Mutex<Settings>>,
    logs: &Arc<Mutex<LogStorage>>,
    radar_data: &Arc<Mutex<RadarData>>,
) -> Option<(u16, String)> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/settings") => {
            let s = settings.lock().ok()?;
            serde_json::to_string(&*s).ok().map(|b| (200, b))
        }

        ("PUT", "/api/settings") => {
            let new: Settings = serde_json::from_str(&req.body).ok()?;
            if let Ok(mut s) = settings.lock() {
                *s = new;
            }
            Some((204, String::new()))
        }

        ("GET", "/api/logs") => {
            let after: usize = query_param(&req.query, "after")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let store = logs.lock().ok()?;
            let entries: Vec<_> = store.entries.iter().skip(after).cloned().collect();
            let resp = serde_json::json!({ "entries": entries, "total": store.entries.len() });
            Some((200, serde_json::to_string(&resp).unwrap()))
        }

        ("GET", "/api/status") => {
            let d = radar_data.lock().ok()?;
            let status = PanelStatus {
                connected: d.connected,
                in_quest: d.quest_elapsed_ms.is_some(),
                has_monster: d.has_monster,
                monster_name: d.monster_name,
                quest_elapsed_ms: d.quest_elapsed_ms,
                quest_name: d.quest_name,
            };
            serde_json::to_string(&status).ok().map(|b| (200, b))
        }

        ("GET", "/api/health") => Some((200, r#"{"ok":true}"#.into())),

        ("GET", "/api/desktop-path") => {
            let path = std::env::var("USERPROFILE")
                .unwrap_or_default();
            Some((200, serde_json::json!({"path": format!("{}\\Desktop", path)}).to_string()))
        }

        ("POST", "/api/logs/clear") => {
            if let Ok(mut store) = logs.lock() {
                store.clear();
            }
            Some((204, String::new()))
        }

        ("POST", "/api/logs/export") => {
            #[derive(serde::Deserialize)]
            struct ExportReq { path: String, content: String }
            if let Ok(req) = serde_json::from_str::<ExportReq>(&req.body) {
                let _ = std::fs::write(&req.path, &req.content);
            }
            Some((204, String::new()))
        }

        _ => None,
    }
}

/// 发送二进制响应（图片等非 JSON 内容）
fn send_binary(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let status_text = match status {
        200 => "200 OK",
        _ => "500 Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Cache-Control: public, max-age=3600\r\n\
         \r\n",
        status_text, content_type, body.len()
    );
    let _ = (stream.write_all(headers.as_bytes()), stream.flush());
    if status == 200 {
        let _ = stream.write_all(body);
    }
}

// ── HTTP 响应 ──────────────────────────────────────────────────

fn send_cors(stream: &mut TcpStream, status: u16, body: &str) {
    let status_text = match status {
        200 => "200 OK",
        204 => "204 No Content",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };

    let headers = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, PUT, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         \r\n",
        status_text,
        body.len()
    );

    let _ = (stream.write_all(headers.as_bytes()), stream.flush());
    if status != 204 {
        let _ = stream.write_all(body.as_bytes());
    }
}
