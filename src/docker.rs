use crate::model::*;
use eframe::egui;
use serde::Deserialize;
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender};

pub fn spawn_worker(ctx: egui::Context, rx: Receiver<DockerCommand>, tx: Sender<DockerEvent>) {
    std::thread::spawn(move || {
        while let Ok(command) = rx.recv() {
            match command {
                DockerCommand::Refresh => match snapshot() {
                    Ok(snapshot) => {
                        let _ = tx.send(DockerEvent::Snapshot(Box::new(snapshot)));
                    }
                    Err(error) => {
                        let _ = tx.send(DockerEvent::Error(error));
                    }
                },
                DockerCommand::LoadLogs(target) => match load_logs(target) {
                    Ok(logs) => {
                        let _ = tx.send(DockerEvent::Logs(logs));
                    }
                    Err(error) => {
                        let _ = tx.send(DockerEvent::Error(error));
                    }
                },
                action => {
                    let result = execute(&action);
                    match snapshot() {
                        Ok(snapshot) => {
                            let _ = tx.send(DockerEvent::Snapshot(Box::new(snapshot)));
                        }
                        Err(error) => {
                            let _ = tx.send(DockerEvent::Error(error));
                        }
                    }
                    match result {
                        Ok(message) => {
                            let _ = tx.send(DockerEvent::Done(message));
                        }
                        Err(error) => {
                            let _ = tx.send(DockerEvent::Error(error));
                        }
                    }
                }
            }
            ctx.request_repaint();
        }
    });
}

pub fn snapshot() -> Result<Snapshot, String> {
    let mut compose = list_compose()?;
    let volumes = list_volumes().unwrap_or_default();
    let containers = list_containers().unwrap_or_default();
    let host_ports = list_host_network_ports(&containers);
    attach_compose_ports(&mut compose, &containers, &host_ports);
    let images = list_images().unwrap_or_default();
    let networks = list_networks().unwrap_or_default();
    let system = list_system_df().unwrap_or_default();
    let engine_version = run_docker(&args(&["info", "--format", "{{.ServerVersion}}"]))
        .map(|value| value.trim().to_string())
        .unwrap_or_default();

    Ok(Snapshot {
        compose,
        volumes,
        containers,
        images,
        networks,
        system,
        engine_version,
        refreshed_at: Some(std::time::Instant::now()),
    })
}

fn list_compose() -> Result<Vec<ComposeProject>, String> {
    let output = run_docker(&args(&["compose", "ls", "-a", "--format", "json"]))?;
    let raw: Vec<ComposeRaw> = serde_json::from_str(output.trim())
        .map_err(|error| format!("Could not read compose projects: {error}"))?;

    Ok(raw
        .into_iter()
        .map(|project| ComposeProject {
            name: project.name,
            status: project.status,
            config_files: project
                .config_files
                .split(',')
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            ports: Vec::new(),
        })
        .collect())
}

fn attach_compose_ports(
    projects: &mut [ComposeProject],
    containers: &[ContainerInfo],
    host_ports: &[(String, Vec<PortLink>)],
) {
    for project in projects {
        let mut ports = Vec::new();
        for container in containers.iter().filter(|container| {
            container.is_running()
                && container.compose_project.as_deref() == Some(project.name.as_str())
        }) {
            ports.extend(parse_published_ports(&container.ports));
        }
        for (project_name, links) in host_ports {
            if project_name == &project.name {
                ports.extend(links.iter().cloned());
            }
        }
        ports.sort();
        ports.dedup();
        project.ports = ports;
    }
}

fn list_host_network_ports(containers: &[ContainerInfo]) -> Vec<(String, Vec<PortLink>)> {
    let running: Vec<&ContainerInfo> = containers
        .iter()
        .filter(|container| container.is_running())
        .collect();
    if running.is_empty() {
        return Vec::new();
    }

    let mut command = args(&[
        "inspect",
        "--format",
        "{{.Name}}|{{.HostConfig.NetworkMode}}|{{json .Config.ExposedPorts}}|{{.State.Running}}",
    ]);
    command.extend(running.iter().map(|container| container.id.clone()));

    let Ok(output) = run_docker(&command) else {
        return Vec::new();
    };

    let mut result: Vec<(String, Vec<PortLink>)> = Vec::new();
    for line in output.lines() {
        let mut parts = line.splitn(4, '|');
        let (Some(name), Some(network), Some(exposed), Some(running)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if network.trim() != "host" || running.trim() != "true" {
            continue;
        }
        let name = name.trim_start_matches('/');
        let Some(container) = containers
            .iter()
            .find(|container| container.is_running() && container.name == name)
        else {
            continue;
        };
        let Some(project) = container.compose_project.as_ref() else {
            continue;
        };
        let ports = parse_exposed_ports(exposed);
        if ports.is_empty() {
            continue;
        }
        if let Some((_, existing)) = result
            .iter_mut()
            .find(|(project_name, _)| project_name == project)
        {
            existing.extend(ports);
            existing.sort();
            existing.dedup();
        } else {
            result.push((project.clone(), ports));
        }
    }
    result
}

fn parse_published_ports(ports: &str) -> Vec<PortLink> {
    let mut parsed = Vec::new();
    for entry in ports.split(',') {
        let entry = entry.trim();
        let Some((host, container)) = entry.split_once("->") else {
            continue;
        };
        let host_port = host
            .rsplit(':')
            .next()
            .and_then(|port| port.split('-').next())
            .and_then(|port| port.parse::<u16>().ok());
        let (container_port, protocol) = container
            .split_once('/')
            .map_or((container, "tcp"), |(port, protocol)| (port, protocol));
        let container_port = container_port
            .split('-')
            .next()
            .and_then(|port| port.parse::<u16>().ok());
        if let (Some(host_port), Some(container_port)) = (host_port, container_port) {
            parsed.push(PortLink {
                host_port,
                container_port,
                protocol: protocol.to_string(),
            });
        }
    }
    parsed.sort();
    parsed.dedup();
    parsed
}

fn parse_exposed_ports(exposed: &str) -> Vec<PortLink> {
    let Ok(exposed) =
        serde_json::from_str::<std::collections::BTreeMap<String, serde_json::Value>>(exposed)
    else {
        return Vec::new();
    };
    let mut ports: Vec<PortLink> = exposed
        .keys()
        .filter_map(|entry| {
            let (port, protocol) = entry.split_once('/')?;
            let port = port.parse::<u16>().ok()?;
            Some(PortLink {
                host_port: port,
                container_port: port,
                protocol: protocol.to_string(),
            })
        })
        .collect();
    ports.sort();
    ports.dedup();
    ports
}

fn list_volumes() -> Result<Vec<VolumeInfo>, String> {
    let output = run_docker(&args(&[
        "system",
        "df",
        "-v",
        "--format",
        "{{json .Volumes}}",
    ]));

    if let Ok(output) = output {
        if let Ok(raw) = serde_json::from_str::<Vec<VolumeRaw>>(output.trim()) {
            return Ok(raw
                .into_iter()
                .map(|volume| VolumeInfo {
                    name: volume.name,
                    driver: volume.driver,
                    size: volume.size,
                    links: volume.links.parse().unwrap_or(0),
                    labels: volume.labels,
                    mountpoint: volume.mountpoint,
                })
                .collect());
        }
    }

    let output = run_docker(&args(&["volume", "ls", "--format", "json"]))?;
    Ok(parse_lines::<VolumeRaw>(&output)
        .into_iter()
        .map(|volume| VolumeInfo {
            name: volume.name,
            driver: volume.driver,
            size: volume.size,
            links: volume.links.parse().unwrap_or(0),
            labels: volume.labels,
            mountpoint: volume.mountpoint,
        })
        .collect())
}

fn list_containers() -> Result<Vec<ContainerInfo>, String> {
    let output = run_docker(&args(&["ps", "-a", "--format", "json"]))?;
    Ok(parse_lines::<ContainerRaw>(&output)
        .into_iter()
        .map(|container| ContainerInfo {
            id: container.id,
            name: container.names.trim_start_matches('/').to_string(),
            image: container.image,
            state: container.state,
            status: container.status,
            ports: container.ports,
            created: container.created_at,
            health: container.health_status,
            compose_project: compose_project(&container.labels),
        })
        .collect())
}

fn list_images() -> Result<Vec<ImageInfo>, String> {
    let output = run_docker(&args(&["images", "--format", "json"]))?;
    Ok(parse_lines::<ImageRaw>(&output)
        .into_iter()
        .map(|image| {
            let dangling = image.repository == "<none>" && image.tag == "<none>";
            ImageInfo {
                id: image.id,
                repository: image.repository,
                tag: image.tag,
                size: image.size,
                created: image.created_since,
                dangling,
            }
        })
        .collect())
}

fn list_networks() -> Result<Vec<NetworkInfo>, String> {
    let output = run_docker(&args(&["network", "ls", "--format", "json"]))?;
    Ok(parse_lines::<NetworkRaw>(&output)
        .into_iter()
        .map(|network| NetworkInfo {
            builtin: matches!(network.name.as_str(), "bridge" | "host" | "none"),
            id: network.id,
            name: network.name,
            driver: network.driver,
            scope: network.scope,
            labels: network.labels,
        })
        .collect())
}

fn list_system_df() -> Result<Vec<SystemDf>, String> {
    let output = run_docker(&args(&["system", "df", "--format", "json"]))?;
    Ok(parse_lines::<SystemDfRaw>(&output)
        .into_iter()
        .map(|entry| SystemDf {
            kind: entry.kind,
            total: entry.total_count.parse().unwrap_or(0),
            active: entry.active.parse().unwrap_or(0),
            size: entry.size,
            reclaimable: entry.reclaimable,
        })
        .collect())
}

fn load_logs(target: LogTarget) -> Result<LogsView, String> {
    match target {
        LogTarget::Compose { name, files } => {
            let arguments = compose_args(&name, &files, &["logs", "--tail=300", "--no-color"]);
            let text = run_docker(&arguments)?;
            Ok(LogsView {
                title: format!("Logs · {name}"),
                text,
            })
        }
        LogTarget::Container { id, name } => {
            let text = run_docker(&args(&["logs", "--tail", "300", &id]))?;
            Ok(LogsView {
                title: format!("Logs · {name}"),
                text,
            })
        }
    }
}

fn execute(command: &DockerCommand) -> Result<String, String> {
    match command {
        DockerCommand::Compose {
            name,
            files,
            action,
        } => {
            let tail: &[&str] = match action {
                ComposeAction::Up => &["up", "-d"],
                ComposeAction::Stop => &["stop"],
                ComposeAction::Start => &["start"],
                ComposeAction::Restart => &["restart"],
                ComposeAction::Down => &["down", "--remove-orphans"],
                ComposeAction::DownVolumes => &["down", "-v", "--remove-orphans"],
            };
            run_docker(&compose_args(name, files, tail))?;
            Ok(format!("Compose {name}: {}", action.label()))
        }
        DockerCommand::VolumeRemove(names) => {
            for name in names {
                run_docker(&args(&["volume", "rm", name]))?;
            }
            Ok(format!("Removed {} volume(s)", names.len()))
        }
        DockerCommand::Container { id, name, action } => {
            let tail: &[&str] = match action {
                ContainerAction::Start => &["start"],
                ContainerAction::Stop => &["stop"],
                ContainerAction::Restart => &["restart"],
                ContainerAction::Remove => &["rm", "-f"],
            };
            let mut command = args(&["container"]);
            command.extend(tail.iter().map(|value| value.to_string()));
            command.push(id.clone());
            run_docker(&command)?;
            Ok(format!("Container {name}: {}", action.label()))
        }
        DockerCommand::ImageRemove(id) => {
            run_docker(&args(&["image", "rm", id]))?;
            Ok(format!("Image removed: {id}"))
        }
        DockerCommand::NetworkRemove(id) => {
            run_docker(&args(&["network", "rm", id]))?;
            Ok(format!("Network removed: {id}"))
        }
        DockerCommand::Prune(kind) => {
            let tail: &[&str] = match kind {
                PruneKind::Containers => &["container", "prune", "-f"],
                PruneKind::Images => &["image", "prune", "-f"],
                PruneKind::Volumes => &["volume", "prune", "-f"],
                PruneKind::Networks => &["network", "prune", "-f"],
                PruneKind::BuildCache => &["builder", "prune", "-f"],
            };
            run_docker(&args(tail))?;
            Ok(format!("Pruned {}", kind.label()))
        }
        DockerCommand::Refresh | DockerCommand::LoadLogs(_) => {
            Err("Internal command routed to the wrong worker branch".to_string())
        }
    }
}

fn compose_args(name: &str, files: &[String], tail: &[&str]) -> Vec<String> {
    let mut arguments = args(&["compose", "-p", name]);
    for file in files {
        arguments.push("-f".to_string());
        arguments.push(file.clone());
    }
    arguments.extend(tail.iter().map(|value| value.to_string()));
    arguments
}

fn run_docker(arguments: &[String]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(arguments)
        .env("DOCKER_HOST", "unix:///var/run/docker.sock")
        .env("DOCKER_CONTEXT", "default")
        .output()
        .map_err(|error| format!("Could not run docker: {error}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err(format!(
            "docker {} failed: {}",
            arguments.join(" "),
            truncate(detail, 2_000)
        ))
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn parse_lines<T>(output: &str) -> Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else {
                serde_json::from_str(line).ok()
            }
        })
        .collect()
}

fn compose_project(labels: &str) -> Option<String> {
    labels.split(',').find_map(|pair| {
        pair.trim()
            .strip_prefix("com.docker.compose.project=")
            .map(ToOwned::to_owned)
    })
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(limit).collect();
        format!("{truncated}…")
    }
}

#[derive(Deserialize, Default)]
struct ComposeRaw {
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "ConfigFiles", default)]
    config_files: String,
}

#[derive(Deserialize, Default)]
struct VolumeRaw {
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "Size", default)]
    size: String,
    #[serde(rename = "Links", default)]
    links: String,
    #[serde(rename = "Labels", default)]
    labels: String,
    #[serde(rename = "Mountpoint", default)]
    mountpoint: String,
}

#[derive(Deserialize, Default)]
struct ContainerRaw {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Names", default)]
    names: String,
    #[serde(rename = "Image", default)]
    image: String,
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "Ports", default)]
    ports: String,
    #[serde(rename = "CreatedAt", default)]
    created_at: String,
    #[serde(rename = "HealthStatus", default)]
    health_status: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

#[derive(Deserialize, Default)]
struct ImageRaw {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Repository", default)]
    repository: String,
    #[serde(rename = "Tag", default)]
    tag: String,
    #[serde(rename = "Size", default)]
    size: String,
    #[serde(rename = "CreatedSince", default)]
    created_since: String,
}

#[derive(Deserialize, Default)]
struct NetworkRaw {
    #[serde(rename = "ID", default)]
    id: String,
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "Scope", default)]
    scope: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_compose_project_from_labels() {
        let labels = "a=b,com.docker.compose.project=db,c=d";
        assert_eq!(compose_project(labels), Some("db".to_string()));
        assert_eq!(compose_project("a=b"), None);
    }

    #[test]
    fn truncates_long_values() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("abcdefghij", 5), "abcde…");
    }

    #[test]
    fn parses_json_lines() {
        let output = concat!(
            "{\"ID\":\"abc\",\"Names\":\"one\"}\n",
            "{\"ID\":\"def\",\"Names\":\"two\"}\n"
        );
        let containers: Vec<ContainerRaw> = parse_lines(output);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[1].id, "def");
        assert_eq!(containers[1].names, "two");
    }

    #[test]
    fn parses_compose_project_array() {
        let raw: Vec<ComposeRaw> = serde_json::from_str(
            r#"[{"Name":"db","Status":"running(1)","ConfigFiles":"/a.yml,/b.yml"}]"#,
        )
        .expect("compose json should parse");
        assert_eq!(raw.len(), 1);
        assert_eq!(raw[0].name, "db");
        assert_eq!(raw[0].config_files, "/a.yml,/b.yml");
    }

    #[test]
    fn parses_published_ports_and_urls() {
        let ports = parse_published_ports(
            "0.0.0.0:3000->3000/tcp, :::3000->3000/tcp, 5432/tcp, \
             127.0.0.1:8080->80/tcp, 0.0.0.0:5353->53/udp",
        );
        assert_eq!(
            ports,
            vec![
                PortLink {
                    host_port: 3000,
                    container_port: 3000,
                    protocol: "tcp".to_string(),
                },
                PortLink {
                    host_port: 5353,
                    container_port: 53,
                    protocol: "udp".to_string(),
                },
                PortLink {
                    host_port: 8080,
                    container_port: 80,
                    protocol: "tcp".to_string(),
                },
            ]
        );
        assert_eq!(ports[0].url().as_deref(), Some("http://127.0.0.1:3000"));
        assert_eq!(ports[1].url(), None);
    }

    #[test]
    fn parses_host_network_exposed_ports() {
        let ports = parse_exposed_ports(r#"{"7456/tcp":{},"53/udp":{}}"#);
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].host_port, 53);
        assert_eq!(ports[1].url().as_deref(), Some("http://127.0.0.1:7456"));
    }

    #[test]
    fn attaches_ports_to_matching_project_only() {
        let mut projects = vec![
            ComposeProject {
                name: "alpha".to_string(),
                ..Default::default()
            },
            ComposeProject {
                name: "beta".to_string(),
                ..Default::default()
            },
        ];
        let containers = vec![ContainerInfo {
            compose_project: Some("alpha".to_string()),
            state: "running".to_string(),
            ports: "0.0.0.0:3000->3000/tcp".to_string(),
            ..Default::default()
        }];
        attach_compose_ports(&mut projects, &containers, &[]);
        assert_eq!(projects[0].ports.len(), 1);
        assert!(projects[1].ports.is_empty());
    }
}

#[derive(Deserialize, Default)]
struct SystemDfRaw {
    #[serde(rename = "Type", default)]
    kind: String,
    #[serde(rename = "TotalCount", default)]
    total_count: String,
    #[serde(rename = "Active", default)]
    active: String,
    #[serde(rename = "Size", default)]
    size: String,
    #[serde(rename = "Reclaimable", default)]
    reclaimable: String,
}
