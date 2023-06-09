use futures::StreamExt;
use ipnet::IpNet;
use log::{error, info};

use std::{
    format,
    net::Ipv4Addr,
    println,
    process::{self, Command},
    str::FromStr,
};
use tun::AsyncDevice;

use pnet::packet::{ip::IpNextHeaderProtocols, ipv4};

pub async fn serve() {
    Gateway::new().serve().await;
}

struct Gateway {
    gateway: Ipv4Addr,
    network: IpNet,
}

impl Gateway {
    fn new() -> Self {
        let network = IpNet::from_str("10.0.0.1/16").unwrap();
        let gateway = match network {
            IpNet::V4(v) => v.addr(),
            IpNet::V6(_) => panic!("not supported yet"),
        };

        Self { gateway, network }
    }

    async fn serve(&self) {
        let dev = self.setup().await;

        let mut stream = dev.into_framed();

        while let Some(packet) = stream.next().await {
            if let Ok(packet) = packet {
                let mut packet = packet.get_bytes().to_vec();
                let v4 = ipv4::Ipv4Packet::new(&mut packet).unwrap();
                let _src = v4.get_source();
                let dst = v4.get_destination();
                let protocol = v4.get_next_level_protocol();
                let process_list = Command::new("sh")
                    .arg("-c")
                    .arg(format!("lsof -Pni | grep {}", dst))
                    .output()
                    .unwrap();
                let std_out = String::from_utf8_lossy(&process_list.stdout);
                let std_err = String::from_utf8_lossy(&process_list.stderr);

                if !process_list.status.success() {
                    println!("{std_err}");
                }
                let process_name = match std_out.split_whitespace().next() {
                    Some(process_name) => process_name,
                    _ => "NO PROCESS(?)",
                };

                match protocol {
                    IpNextHeaderProtocols::Icmp => println!("{process_name} -> ICMP to {:?}", dst),
                    IpNextHeaderProtocols::Udp => println!("{process_name} -> UDP to {:?}", dst),
                    IpNextHeaderProtocols::Tcp => println!("{process_name} -> TCP to {:?}", dst),
                    _ => println!("OTHER"),
                }
            }
        }
    }

    async fn setup(&self) -> AsyncDevice {
        info!("gateway addr: {}", self.network.addr());
        let mut config = tun::Configuration::default();
        config
            .layer(tun::Layer::L3)
            .address(self.network.addr())
            .destination(self.gateway)
            .netmask(self.network.netmask())
            .up();

        let dev = match tun::create_as_async(&config) {
            Ok(dev) => dev,
            Err(e) => {
                error!("create tun failed, err: {:?}", e);
                process::exit(1);
            }
        };

        let gateway = &self.gateway.to_string();

        //reset dns when terminate
        #[cfg(target_os = "macos")]
        tokio::spawn(async {
            use tokio::signal;
            if let Ok(_) = signal::ctrl_c().await {
                let _ = Command::new("networksetup")
                    .args(["-setdnsservers", "Wi-Fi", "empty"])
                    .output();

                let _ = Command::new("route")
                    .args(["-n", "delete", "default"])
                    .output();
                let _ = Command::new("route")
                    .args(["-n", "add", "default", "192.168.1.1"])
                    .output();
                process::exit(0);
            }
        });

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("route").args(["delete", "default"]).output();

            let _ = Command::new("route")
                .args(["-n", "add", "default", gateway])
                .output();
        }

        dev
    }
}
