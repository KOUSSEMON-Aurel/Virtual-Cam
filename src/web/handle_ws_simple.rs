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
                            // 1. Envoyer vers UDP (legacy)
                            let _ = udp_socket.send_to(&bin, target_addr).await;
                            
                            // 2. Diffuser vers le dashboard
                            let _ = video_tx.send(bin.to_vec());
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
