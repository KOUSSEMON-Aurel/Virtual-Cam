#![feature(portable_simd)]
#![allow(unused)]

use mimalloc::MiMalloc;
use std::net::UdpSocket;
use tokio::io::AsyncWriteExt;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod net;
mod memory;
mod web;
mod metrics;
mod pipeline;
mod v4l2;
mod codec;

use local_ip_address::local_ip;
use qrcode::QrCode;
use qrcode::render::unicode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 PHONECAM ULTIMATE — VERSION MVP");
    
    // 0. Initialisation des métriques
    let metrics = metrics::ServerMetrics::new();
    let metrics_for_udp = metrics.clone();
    let metrics_for_web = metrics.clone();

    // 1. Détecter l'IP locale
    let my_ip = local_ip().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
    let https_url = format!("https://{}", my_ip);  // Port 443 par défaut
    
    println!("✓ Ton IP locale : {}", my_ip);
    println!("✓ URL HTTPS : {}", https_url);
    println!("✓ Dashboard PC : {}/dashboard", https_url);
    
    // 2. Générer le QR Code pour le smartphone (HTTPS)
    let code = QrCode::new(https_url.as_bytes())?;
    let image = code.render::<unicode::Dense1x2>().build();
    println!("\n📱 SCANNE MOI POUR CONNECTER TON SMARTPHONE :\n{}", image);

    // 3. Lancer le serveur UDP en tâche de fond
    let udp_socket = std::net::UdpSocket::bind("0.0.0.0:9999")?;
    udp_socket.set_nonblocking(true)?;
    
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        let socket = tokio::net::UdpSocket::from_std(udp_socket).unwrap();
        
        loop {
            let (len, src) = socket.recv_from(&mut buf).await.unwrap();
            if let Some(_header) = net::protocol::Header::parse(&buf[..len]) {
                metrics_for_udp.record_packet(len as u64);
                
                let count = metrics_for_udp.packet_count.load(std::sync::atomic::Ordering::Relaxed);
                if count % 100 == 0 {
                    println!("📦 {} paquets reçus | Client: {}", count, src);
                }
            }
        }
    });

    // 4. Initialiser la pipeline vidéo (décodage + V4L2)
    // TEMPORAIREMENT DÉSACTIVÉ : mmap() ne fonctionne pas directement avec v4l2loopback
    // let pipeline = pipeline::Pipeline::new(10)?; // /dev/video10 (v4l2loopback)
    
    // 5. Lancer le serveur Web (bloquant)
    web::server::start_server_without_pipeline(8080, 9999, metrics_for_web).await;
    
    Ok(())
}
