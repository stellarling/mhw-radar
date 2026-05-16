//! HTTP API 服务器（127.0.0.1:17320）
//!
//! 为 Tauri 工具面板提供 REST 接口，通过 JSON 交换设置/日志/状态。
//! 纯 std::net 手撸，无 tokio 依赖，单线程 per-connection 模型。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::log::ConnectionLogStorage;
use crate::log::LogStorage;
use crate::types::{PanelStatus, RadarData, Settings};

/// 单次 HTTP 请求体上限。
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024 * 1024;

/// 启动 HTTP API 服务器（在后台线程运行）
pub fn start_server(
    port: u16,
    settings: Arc<Mutex<Settings>>,
    logs: Arc<Mutex<LogStorage>>,
    connection_logs: Arc<Mutex<ConnectionLogStorage>>,
    radar_data: Arc<Mutex<RadarData>>,
) {
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("IPC server: failed to bind port");

    println!("[IPC] API server listening on http://{}", addr);

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let (s, l, c, r) = (
                    settings.clone(),
                    logs.clone(),
                    connection_logs.clone(),
                    radar_data.clone(),
                );
                thread::spawn(move || handle(stream, &s, &l, &c, &r));
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

    // 不 trim body。导出日志时尾部换行属于用户内容，不能在 IPC 层隐式修改。
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
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

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn extract_content_length(header: &str) -> Option<usize> {
    header
        .lines()
        .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
        .and_then(|l| l.split_once(':')?.1.trim().parse::<usize>().ok())
}

fn json_error(message: impl AsRef<str>) -> String {
    serde_json::json!({ "ok": false, "error": message.as_ref() }).to_string()
}

// ── 路由 ────────────────────────────────────────────────────────

fn handle(
    mut stream: TcpStream,
    settings: &Arc<Mutex<Settings>>,
    logs: &Arc<Mutex<LogStorage>>,
    connection_logs: &Arc<Mutex<ConnectionLogStorage>>,
    radar_data: &Arc<Mutex<RadarData>>,
) {
    let mut buf = vec![0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };
    buf.truncate(n);

    let header_end = match find_header_end(&buf) {
        Some(pos) => pos,
        None => return send_cors(&mut stream, 400, &json_error("Bad Request")),
    };

    let header = String::from_utf8_lossy(&buf[..header_end]);
    let content_len = extract_content_length(&header).unwrap_or(0);

    if content_len > MAX_REQUEST_BODY_BYTES {
        return send_cors(
            &mut stream,
            413,
            &json_error(format!(
                "request body too large: {} bytes, max {} bytes",
                content_len, MAX_REQUEST_BODY_BYTES
            )),
        );
    }

    let total_needed = header_end + content_len;
    if buf.len() < total_needed {
        buf.resize(total_needed, 0);
        let mut total = n;
        while total < total_needed {
            match stream.read(&mut buf[total..total_needed]) {
                Ok(0) => break,
                Ok(r) => total += r,
                Err(_) => break,
            }
        }
        buf.truncate(total);
    }

    if buf.len() < total_needed {
        return send_cors(
            &mut stream,
            400,
            &json_error(format!(
                "incomplete request body: received {} bytes, expected {} bytes",
                buf.len().saturating_sub(header_end),
                content_len
            )),
        );
    }

    let raw = String::from_utf8_lossy(&buf).to_string();
    let req = match parse_request(&raw) {
        Some(r) => r,
        None => return send_cors(&mut stream, 400, &json_error("Bad Request")),
    };

    if req.method == "OPTIONS" {
        return send_cors(&mut stream, 204, "");
    }

    // 二进制资源（logo）走独立路径
    if req.path == "/api/resources/logo" && req.method == "GET" {
        let png = include_bytes!("../resources/title-zh-en.png");
        return send_binary(&mut stream, 200, "image/png", png);
    }

    match route(&req, settings, logs, connection_logs, radar_data) {
        Some((status, body)) => send_cors(&mut stream, status, &body),
        None => send_cors(&mut stream, 404, &json_error("Not Found")),
    }
}

fn route(
    req: &Request,
    settings: &Arc<Mutex<Settings>>,
    logs: &Arc<Mutex<LogStorage>>,
    connection_logs: &Arc<Mutex<ConnectionLogStorage>>,
    radar_data: &Arc<Mutex<RadarData>>,
) -> Option<(u16, String)> {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/settings") => {
            let s = settings.lock().ok()?;
            serde_json::to_string(&*s).ok().map(|b| (200, b))
        }

        ("PUT", "/api/settings") => {
            let new: Settings = match serde_json::from_str(&req.body) {
                Ok(v) => v,
                Err(e) => return Some((400, json_error(format!("invalid settings JSON: {}", e)))),
            };
            if let Ok(mut s) = settings.lock() {
                *s = new;
            } else {
                return Some((500, json_error("failed to lock settings")));
            }
            Some((204, String::new()))
        }

        ("GET", "/api/logs") => {
            let round: Option<usize> = query_param(&req.query, "round")
                .and_then(|v| v.parse().ok());
            let store = logs.lock().ok()?;
            let total_rounds = store.round_count();
            let resp = if let Some(r) = round {
                let entries: Vec<_> = store.get_round(r).cloned()
                    .map(|v| v.iter().cloned().collect())
                    .unwrap_or_default();
                serde_json::json!({ "entries": entries, "round": r, "total_rounds": total_rounds })
            } else {
                // 无条件返回全部（用于导出全部）
                let entries = store.all_entries();
                serde_json::json!({ "entries": entries, "round": 0, "total_rounds": total_rounds })
            };
            Some((200, serde_json::to_string(&resp).unwrap()))
        }

        ("GET", "/api/logs/round-count") => {
            let store = logs.lock().ok()?;
            Some((200, serde_json::json!({ "total_rounds": store.round_count() }).to_string()))
        }

        ("GET", "/api/status") => {
            let d = radar_data.lock().ok()?;
            let status = PanelStatus {
                connected: d.connected,
                in_quest: d.in_quest,
                has_monster: d.has_monster,
                monster_name: d.monster_name,
                quest_elapsed_ms: d.quest_elapsed_ms,
                quest_name: d.quest_name,
                connection_state: d.connection_state.clone(),
                pid: d.pid,
                module_base: d.module_base,
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
                Some((204, String::new()))
            } else {
                Some((500, json_error("failed to lock log storage")))
            }
        }

        ("POST", "/api/logs/export") => {
            #[derive(serde::Deserialize)]
            struct ExportReq {
                path: String,
                content: String,
            }

            let export_req = match serde_json::from_str::<ExportReq>(&req.body) {
                Ok(v) => v,
                Err(e) => {
                    return Some((
                        400,
                        json_error(format!("invalid export JSON: {}", e)),
                    ))
                }
            };

            if export_req.path.trim().is_empty() {
                return Some((400, json_error("export path is empty")));
            }

            let path = PathBuf::from(export_req.path);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return Some((
                            500,
                            json_error(format!(
                                "failed to create export directory '{}': {}",
                                parent.display(),
                                e
                            )),
                        ));
                    }
                }
            }

            match std::fs::write(&path, export_req.content.as_bytes()) {
                Ok(_) => Some((
                    200,
                    serde_json::json!({
                        "ok": true,
                        "path": path.to_string_lossy(),
                    })
                    .to_string(),
                )),
                Err(e) => Some((
                    500,
                    json_error(format!("failed to write export file '{}': {}", path.display(), e)),
                )),
            }
        }

        ("GET", "/api/connection-logs") => {
            let store = connection_logs.lock().ok()?;
            let entries = store.all_entries();
            drop(store);
            Some((
                200,
                serde_json::json!({ "entries": entries }).to_string(),
            ))
        }

        ("POST", "/api/connection-logs/clear") => {
            if let Ok(mut store) = connection_logs.lock() {
                store.clear();
                Some((204, String::new()))
            } else {
                Some((500, json_error("failed to lock connection log storage")))
            }
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
        413 => "413 Payload Too Large",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    };

    let headers = format!(
        "HTTP/1.1 {}\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, PUT, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         \r\n",
        status_text,
        body.as_bytes().len()
    );

    let _ = (stream.write_all(headers.as_bytes()), stream.flush());
    if status != 204 {
        let _ = stream.write_all(body.as_bytes());
    }
}
