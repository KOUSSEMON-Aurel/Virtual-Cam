use axum::{
    routing::get,
    Router,
    extract::ws::{WebSocketUpgrade, WebSocket, Message},
    response::IntoResponse,
};
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

pub async fn start_server_without_pipeline(http_port: u16, udp_port: u16, metrics: Arc<crate::metrics::ServerMetrics>) {
    let (tx, _rx) = broadcast::channel::<Vec<u8>>(16);
    let video_tx = Arc::new(tx);
    
    let metrics_filter = metrics.clone();
    let video_tx_filter = video_tx.clone();
    
    let app = Router::new()
        .route("/", get(|| async {
            axum::response::Html(include_str!("../../web/index.html"))
        }))
        .route("/app.js", get(|| async {
            (
                [(axum::http::header::CONTENT_TYPE, "application/javascript")],
                include_str!("../../web/app.js")
            )
        }))
        .route("/dashboard", get(|| async {
            axum::response::Html(include_str!("../../web/dashboard.html"))
        }))
        .route("/stats", get(move |ws: WebSocketUpgrade| {
            let m = metrics_filter.clone();
            async move {
                ws.on_upgrade(move |socket| handle_stats_ws(socket, m))
            }
        }))
        .route("/video", get(move |ws: WebSocketUpgrade| {
            let tx = video_tx_filter.clone();
            async move {
                ws.on_upgrade(move |socket| handle_video_ws(socket, tx))
            }
        }))
        .route("/raw", get(move |ws: WebSocketUpgrade| {
            let tx = video_tx.clone();
            let m = metrics.clone();
            async move {
                ws.on_upgrade(move |socket| handle_ws_simple(socket, udp_port, tx, m))
            }
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    
    println!("⚠️  Mode HTTP - L'accès caméra nécessite HTTPS sur mobile !");
    println!("🌐 Serveur Web : http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    // Axum 0.7+ gère automatiquement TCP_NODELAY
    axum::serve(listener, app).await.unwrap();
}

pub async fn start_server(http_port: u16, udp_port: u16, metrics: Arc<crate::metrics::ServerMetrics>, pipeline: Arc<crate::pipeline::Pipeline>) {
    let (tx, _rx) = broadcast::channel::<Vec<u8>>(16);
    let video_tx = Arc::new(tx);
    
    let metrics_filter = metrics.clone();
    let video_tx_filter = video_tx.clone();
    
    let app = Router::new()
        // ... (routes inchangées jusqu'à /raw)
        .route("/raw", get(move |ws: WebSocketUpgrade| {
            let tx = video_tx.clone();
            let m = metrics.clone();
            let p = pipeline.clone();
            async move {
                ws.on_upgrade(move |socket| handle_ws(socket, udp_port, tx, m, p))
            }
        }));

    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    
    println!("⚠️  Mode HTTP - L'accès caméra nécessite HTTPS sur mobile !");
    println!("🌐 Serveur Web : http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_stats_ws(mut socket: WebSocket, metrics: Arc<crate::metrics::ServerMetrics>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
    loop {
        interval.tick().await;
        let snapshot = metrics.snapshot();
        let json = serde_json::to_string(&snapshot).unwrap();
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

async fn handle_video_ws(mut socket: WebSocket, tx: Arc<broadcast::Sender<Vec<u8>>>) {
    let mut rx = tx.subscribe();
    while let Ok(data) = rx.recv().await {
        if socket.send(Message::Binary(data.into())).await.is_err() {
            break;
        }
    }
}

async fn handle_ws(
    mut socket: WebSocket, 
    udp_port: u16, 
    video_tx: Arc<broadcast::Sender<Vec<u8>>>, 
    metrics: Arc<crate::metrics::ServerMetrics>,
    pipeline: Arc<crate::pipeline::Pipeline>
) {
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let target_addr: SocketAddr = format!("127.0.0.1:{}", udp_port).parse().unwrap();

    let mut buf = vec![0u8; 65536];
    
    loop {
        tokio::select! {
            msg = socket.recv() => {
                if let Some(Ok(msg)) = msg {
                    match msg {
                        Message::Binary(bin) => {
                            // 1. Envoyer vers UDP (legacy)
                            let _ = udp_socket.send_to(&bin, target_addr).await;
                            
                            // 2. Diffuser vers le dashboard
                            let _ = video_tx.send(bin.to_vec());
                            
                            // 3. Passer dans la pipeline de décodage + V4L2
                            let p = pipeline.clone();
                            let b = bin.clone();
                            let m = metrics.clone();
                            tokio::spawn(async move {
                                let snapshot = m.snapshot();
                                let _ = p.process_chunk(&b, snapshot.width as usize, snapshot.height as usize).await;
                            });
                        }
                        Message::Text(text) => {
                            // Métadonnées JSON
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                if val["type"] == "metadata" {
                                    if let (Some(w), Some(h)) = (val["width"].as_u64(), val["height"].as_u64()) {
                                        metrics.update_resolution(w, h);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
// ...
            res = udp_socket.recv_from(&mut buf) => {
                if let Ok((len, _)) = res {
                    if socket.send(Message::Binary(buf[..len].to_vec())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

async fn handle_ws_simple(
    mut socket: WebSocket, 
    udp_port: u16, 
    video_tx: Arc<broadcast::Sender<Vec<u8>>>, 
    metrics: Arc<crate::metrics::ServerMetrics>
) {
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await.unwrap();
    let target_addr: SocketAddr = format!("127.0.0.1:{}", udp_port).parse().unwrap();

    let mut buf = vec![0u8; 65536];
    
    loop {
        tokio::select! {
            msg = socket.recv() => {
                if let Some(Ok(msg)) = msg {
                    match msg {
                        Message::Binary(bin) => {
                            // Log toutes les 60 frames
                            static FRAME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                            let count = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if count % 60 == 0 {
                                println!("[WS] 📹 {} chunks reçus ({} bytes), broadcast vers dashboard", count, bin.len());
                            }
                            
                            let _ = udp_socket.send_to(&bin, target_addr).await;
                            let _ = video_tx.send(bin.to_vec());
                        }
                        Message::Text(text) => {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                if val["type"] == "metadata" {
                                    if let (Some(w), Some(h)) = (val["width"].as_u64(), val["height"].as_u64()) {
                                        metrics.update_resolution(w, h);
                                    }
                                }
                            }
                            // Relayer le message texte au dashboard (en binaire pour le channel)
                            let _ = video_tx.send(text.into_bytes());
                        }
                        _ => {}
                    }
                } else {
                    break;
                }
            }
            res = udp_socket.recv_from(&mut buf) => {
                if let Ok((len, _)) = res {
                    if socket.send(Message::Binary(buf[..len].to_vec())).await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}
