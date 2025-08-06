use common::TaskInstance;
use config_manager::get_config;

pub enum AccessInfo {
    HttpUrl(String),
    TcpCmd(String),
}

pub fn generate_access_info(inst: &TaskInstance) -> AccessInfo {
    let cfg = get_config();
    let task_cfg = cfg
        .tasks
        .get(&inst.task_name)
        .unwrap_or_else(|| cfg.tasks.get("_default").unwrap());
    let host = match cfg.routing.variant.as_str() {
        "traefik" => format!(
            "{}.{}",
            inst.container_id,
            cfg.routing.traefik_domain
        ),
        _ => cfg.routing.domain.clone(),
    };

    match (cfg.routing.variant.as_str(), task_cfg.protocol.as_str()) {
        ("port", "http") => {
            let url = format!("http://{}:{}", host, inst.port);
            AccessInfo::HttpUrl(url)
        }
        ("port", "tcp") => {
            let cmd = format!("nc {} {}", host, inst.port);
            AccessInfo::TcpCmd(cmd)
        }

        ("traefik", "http") => {
            // assume entrypoint "web" is on port 80
            let url = format!("http://{}/", host);
            AccessInfo::HttpUrl(url)
        }
        ("traefik", "tcp") => {
            let entry_port = match cfg.routing.tcp_entry.as_str() {
                "tcp" => 9000,
                other => other.parse().unwrap_or(9000),
            };
            let cmd = format!("nc {} {}", host, entry_port);
            AccessInfo::TcpCmd(cmd)
        }

        _ => {
            let url = format!("http://{}:{}", host, inst.port);
            AccessInfo::HttpUrl(url)
        }
    }
}
