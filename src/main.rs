// Yeahbut December 2023

use std::error::Error;

use purple_cello_mc_protocol::encrypt;

mod status_handle;
mod login_handle;
mod client;
mod listener;
mod whitelist;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let private_key = encrypt::generate_rsa_keys()?;
    let offline_info = listener::ProxyInfo{
        proxy_addr: "127.0.0.1".to_string(),
        proxy_port: 25565,
        backend_addr: "127.0.0.1".to_string(),
        backend_port: 25564,
        private_key: private_key.clone(),
        online_status: listener::OnlineStatus::Offline,
        authentication_method: listener::AuthenticationMethod::None,
        whitelist: whitelist::Whitelist::WhitelistOpen(
            whitelist::WhitelistOpen{}),
    };
    let online_info = listener::ProxyInfo{
        proxy_addr: "127.0.0.1".to_string(),
        proxy_port: 25566,
        backend_addr: "127.0.0.1".to_string(),
        backend_port: 25564,
        private_key: private_key.clone(),
        online_status: listener::OnlineStatus::Online,
        authentication_method: listener::AuthenticationMethod::Mojang,
        whitelist: whitelist::Whitelist::WhitelistFile(
            whitelist::WhitelistFile::new("./whitelist.json".to_string())),
    };

    let listener_offline: listener::TcpListenerWrapper =
        listener::TcpListenerWrapper::bind(offline_info).await?;
    let listener_online: listener::TcpListenerWrapper =
        listener::TcpListenerWrapper::bind(online_info).await?;

    println!("Proxy listening on port 25565 and 25566...");

    let handle_offline = tokio::spawn(async move{
        while let Ok((client_socket, _)) = listener_offline
            .listener.accept().await {
                tokio::spawn(client::handle_client(
                    client_socket, listener_offline.info.clone()));
        }
    });
    let handle_online = tokio::spawn(async move{
        while let Ok((client_socket, _)) = listener_online
            .listener.accept().await {
                tokio::spawn(client::handle_client(
                    client_socket, listener_online.info.clone()));
        }
    });

    tokio::try_join!(handle_offline, handle_online)?;

    Ok(())
}
