use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Route {
    pub path: String,
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub delay: Duration,
}

pub struct HttpTestServer {
    base_url: String,
    stop: Arc<AtomicBool>,
    request_counts: Arc<Mutex<std::collections::HashMap<String, usize>>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl HttpTestServer {
    pub fn new(routes: Vec<Route>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();

        let base_url = format!("http://{}", addr);
        let stop = Arc::new(AtomicBool::new(false));
        let request_counts = Arc::new(Mutex::new(std::collections::HashMap::new()));

        let thread_stop = Arc::clone(&stop);
        let thread_counts = Arc::clone(&request_counts);

        let routes = Arc::new(routes);
        let thread_routes = Arc::clone(&routes);

        let handle = std::thread::spawn(move || {
            run_server(listener, addr, thread_routes, thread_stop, thread_counts)
        });

        Self {
            base_url,
            stop,
            request_counts,
            handle: Some(handle),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub fn requests_for_path(&self, path: &str) -> usize {
        self.request_counts
            .lock()
            .ok()
            .and_then(|counts| counts.get(path).copied())
            .unwrap_or(0)
    }

    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_server(
    listener: TcpListener,
    addr: SocketAddr,
    routes: Arc<Vec<Route>>,
    stop: Arc<AtomicBool>,
    request_counts: Arc<Mutex<std::collections::HashMap<String, usize>>>,
) {
    let _ = addr;
    while !stop.load(Ordering::Relaxed) {
        let (mut stream, _peer) = match listener.accept() {
            Ok(pair) => pair,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(_) => break,
        };

        let routes = Arc::clone(&routes);
        let request_counts = Arc::clone(&request_counts);
        std::thread::spawn(move || handle_connection(&mut stream, &routes, &request_counts));
    }
}

fn handle_connection(
    stream: &mut (impl Read + Write),
    routes: &[Route],
    request_counts: &Arc<Mutex<std::collections::HashMap<String, usize>>>,
) {
    let mut req = Vec::new();
    let mut buf = [0u8; 1024];
    while !req.ends_with(b"\r\n\r\n") && req.len() < 32 * 1024 {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        req.extend_from_slice(&buf[..n]);
    }

    let Some((method, path)) = parse_request(&req) else {
        let _ = write_response(stream, 400, "text/plain", b"bad request", false);
        return;
    };

    if let Ok(mut counts) = request_counts.lock() {
        let entry = counts.entry(path.to_owned()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    let route = routes.iter().find(|route| route.path == path);
    match route {
        Some(route) => {
            std::thread::sleep(route.delay);
            let head_only = method == "HEAD";
            let _ = write_response(
                stream,
                route.status,
                &route.content_type,
                &route.body,
                head_only,
            );
        }
        None => {
            let _ = write_response(stream, 404, "text/plain", b"not found", false);
        }
    }
}

fn parse_request(request: &[u8]) -> Option<(&str, &str)> {
    let req = std::str::from_utf8(request).ok()?;
    let line = req.lines().next()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let raw_target = parts.next()?;
    if method != "GET" && method != "HEAD" {
        return None;
    }
    Some((method, normalize_target(raw_target)))
}

fn normalize_target(target: &str) -> &str {
    let Some(scheme_end) = target.find("://") else {
        return target;
    };
    let after_scheme = &target[scheme_end.saturating_add(3)..];
    let Some(path_start) = after_scheme.find('/') else {
        return "/";
    };
    &after_scheme[path_start..]
}

fn write_response(
    stream: &mut impl Write,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let reason = reason_phrase(status);
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
        status,
        reason,
        body.len(),
        content_type
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    }
}
