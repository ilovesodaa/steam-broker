use std::collections::HashMap;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use steamworks::SteamId;
use steamworks::{Client, ServerListCallbacks, User};

// TODO:
// 1. Move steam iniitialization to sb_connect handler
// 2. ... and subsequently stop steam in sb_terminate
// 3. rename sb_terminate for something 10 byte size like sb_connect :)

#[derive(Default)]
struct BrokerState {
    gamedir: String,
}

fn parse_args(buf: &[u8]) -> Option<Vec<&str>> {
    let text = std::str::from_utf8(buf).ok()?.trim();
    Some(text.split_whitespace().collect())
}

fn gamedir_to_app_id(gamedir: &str) -> u32 {
    match gamedir {
        "cstrike" => 10,
        "tfc" => 20,
        "dod" => 30,
        "dmc" => 40,
        "gearbox" => 50,
        "ricochet" => 60,
        "valve" => 70,
        "czero" => 80,
        "czeror" => 100,
        _ => 70,
    }
}

fn build_server_list_response(key: u32, servers: &[SocketAddrV4]) -> Vec<u8> {
    let mut response = Vec::with_capacity(11 + 6 + servers.len() * 6 + 6);

    // Same layout that the engine already uses for normal master server packets:
    // -1, "f\n", 0x7f, <request key>, reserved, then ipv4:port entries and a null terminator.
    response.extend_from_slice(b"\xff\xff\xff\xfff\n");
    response.push(0x7f);
    response.extend_from_slice(&key.to_le_bytes());
    response.push(0);

    for server in servers {
        response.extend_from_slice(&server.ip().octets());
        response.extend_from_slice(&server.port().to_be_bytes());
    }

    response.extend_from_slice(&[0, 0, 0, 0]);
    response.extend_from_slice(&0u16.to_be_bytes());
    response
}

fn send_server_list(sock: &UdpSocket, from: SocketAddr, key: u32, servers: Vec<SocketAddrV4>) {
    let response = build_server_list_response(key, &servers);

    if let Err(err) = sock.send_to(&response, from) {
        println!("error sending server list to {from}: {err}");
    }
}

fn handle_connect(user: &User, args: &[&str], from: SocketAddr, sock: &UdpSocket) {
    if args.len() < 5 {
        println!("handle_connect: not enough arguments");
        return;
    }

    let serveradr: SocketAddrV4 = match args[1].parse() {
        Ok(addr) => addr,
        Err(err) => {
            println!("handle_connect: can't parse ip addr: {err}");
            return;
        }
    };

    let game_server_steam_id = SteamId::from_raw(match args[2].parse() {
        Ok(steam_id) => steam_id,
        Err(err) => {
            println!("handle_connect: can't parse steam id: {err}");
            return;
        }
    });

    let secure: bool = match args[3].parse() {
        Ok(secure) => secure,
        Err(err) => {
            println!("handle_connect: can't parse secure: {err}");
            return;
        }
    };

    let challenge: i32 = match args[4].parse() {
        Ok(challenge) => challenge,
        Err(err) => {
            println!("handle_connect: can't parse challenge: {err}");
            return;
        }
    };

    println!(
        "initiate_game_connection: {serveradr} {:?} {secure} {challenge}",
        game_server_steam_id.raw()
    );
    let ticket = user.initiate_game_connection(
        game_server_steam_id,
        serveradr.ip().to_bits(),
        serveradr.port(),
        secure,
    );

    println!("steam ticket size: {:?}, sending to {from}", ticket.len());
    println!("ticket data: {:?}", ticket);

    // sb_connect\n<4 byte challenge><8 byte steamid><unsigned 4 byte len><len bytes ticket>
    let mut response = Vec::from(b"\xff\xff\xff\xffsb_connect\n".as_ref());
    response.extend(challenge.to_le_bytes());
    response.extend(user.steam_id().raw().to_le_bytes());
    response.extend((ticket.len() as u32).to_le_bytes());
    response.extend(ticket);

    if let Err(err) = sock.send_to(&response, from) {
        println!("error sending: {err}");
    }
}

fn handle_disconnect(user: &User, args: &[&str]) {
    if args.len() < 2 {
        println!("handle_disconnect: not enough arguments");
        return;
    }

    let serveradr: SocketAddrV4 = match args[1].parse() {
        Ok(addr) => addr,
        Err(err) => {
            println!("handle_disconnect: can't parse ip addr: {err}");
            return;
        }
    };

    user.terminate_game_connection(serveradr.ip().to_bits(), serveradr.port());
}

fn handle_gamedir(state: &Arc<Mutex<BrokerState>>, args: &[&str]) {
    if args.len() < 2 {
        println!("handle_gamedir: not enough arguments");
        return;
    }

    let gamedir = args[1].to_ascii_lowercase();
    state.lock().unwrap().gamedir = gamedir.clone();

    println!("current game directory: {gamedir}");
}

fn handle_terminate(state: &Arc<Mutex<BrokerState>>) {
    state.lock().unwrap().gamedir.clear();
    println!("cleared current game directory");
}

fn handle_servers(
    client: &Client,
    state: &Arc<Mutex<BrokerState>>,
    args: &[&str],
    from: SocketAddr,
    sock: &UdpSocket,
) {
    if args.len() < 2 {
        println!("handle_servers: not enough arguments");
        return;
    }

    let key: u32 = match args[1].parse() {
        Ok(key) => key,
        Err(err) => {
            println!("handle_servers: can't parse request key: {err}");
            return;
        }
    };

    let gamedir = {
        let state = state.lock().unwrap();
        if state.gamedir.is_empty() {
            String::from("valve")
        } else {
            state.gamedir.clone()
        }
    };

    let app_id = gamedir_to_app_id(&gamedir);
    let reply_socket = match sock.try_clone() {
        Ok(sock) => sock,
        Err(err) => {
            println!("handle_servers: failed to clone UDP socket: {err}");
            send_server_list(sock, from, key, Vec::new());
            return;
        }
    };
    let callback_socket = match reply_socket.try_clone() {
        Ok(sock) => sock,
        Err(err) => {
            println!("handle_servers: failed to clone callback socket: {err}");
            send_server_list(&reply_socket, from, key, Vec::new());
            return;
        }
    };

    let expected_gamedir = gamedir.clone();
    let callbacks = ServerListCallbacks::new(
        Box::new(|_request, _server| {}),
        Box::new(|_request, _server| {}),
        Box::new(move |request, response| {
            let mut request = request.lock().unwrap();
            let mut unique_servers = HashSet::new();
            let count = request.get_server_count().unwrap_or(0);

            for server_index in 0..count {
                let Ok(details) = request.get_server_details(server_index) else {
                    continue;
                };

                if !details.successful_response {
                    continue;
                }

                if !expected_gamedir.is_empty()
                    && !details.game_dir.eq_ignore_ascii_case(&expected_gamedir)
                {
                    continue;
                }

                let port = if details.connection_port != 0 {
                    details.connection_port
                } else {
                    details.query_port
                };

                if port == 0 {
                    continue;
                }

                unique_servers.insert(SocketAddrV4::new(details.addr, port));
            }

            let _ = request.release();
            drop(request);

            let mut servers: Vec<_> = unique_servers.into_iter().collect();
            servers.sort_by_key(|server| (u32::from(*server.ip()), server.port()));

            println!(
                "steam query finished for {expected_gamedir} (appid {app_id}) with {response:?}, {} servers",
                servers.len()
            );
            send_server_list(&callback_socket, from, key, servers);
        }),
    );

    let filters = HashMap::new();
    match client
        .matchmaking_servers()
        .internet_server_list(app_id, &filters, callbacks)
    {
        Ok(_) => {
            println!("requesting steam internet server list for {gamedir} (appid {app_id})");
        }
        Err(()) => {
            println!("handle_servers: failed to request steam server list");
            send_server_list(&reply_socket, from, key, Vec::new());
        }
    }
}

fn main() {
    println!("Welcome to Steam Broker!");

    println!("Initializing Steam...");

    let client = Client::init().unwrap();

    let utils = client.utils();
    println!("Utils:");
    println!("AppId: {:?}", utils.app_id());

    let user = client.user();
    println!("User:");
    println!("SteamID: {:?}", user.steam_id());

    let addr = "127.0.0.1:27420";
    let sock = match UdpSocket::bind(addr) {
        Ok(x) => x,
        Err(e) => {
            println!("Error creating socket: {:?}", e);
            std::process::exit(1);
        }
    };
    sock.set_read_timeout(Some(Duration::from_millis(50))).unwrap();
    println!("Started UDP server at {addr}");

    let state = Arc::new(Mutex::new(BrokerState::default()));

    loop {
        client.run_callbacks();

        let mut buf = [0; 1024];
        let (n, from) = match sock.recv_from(&mut buf) {
            Ok(x) => x,
            Err(e) => match e.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => continue,
                _ => break,
            },
        };

        let Some(args) = parse_args(&buf[..n]) else {
            println!("Unknown packet: {:?}", &buf[..n.min(10)]);
            continue;
        };

        match args.first().copied() {
            Some("sb_connect") => {
                println!("got sb_connect");
                handle_connect(&user, &args, from, &sock);
            }
            Some("sb_disconnect") => {
                println!("got sb_disconnect");
                handle_disconnect(&user, &args);
            }
            Some("sb_gamedir") => {
                println!("got sb_gamedir");
                handle_gamedir(&state, &args);
            }
            Some("sb_servers") => {
                println!("got sb_servers");
                handle_servers(&client, &state, &args, from, &sock);
            }
            Some("sb_terminate") => {
                println!("got sb_terminate");
                handle_terminate(&state);
            }
            _ => {
                println!("Unknown packet: {:?}", &buf[..n.min(10)]);
            }
        }
    }
}
