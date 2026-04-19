//! # hpotter docker
//!
//! This module provides a wrapper around the bollard::Docker API
//! and provides Result types that work with `anyhow` for more
//! robust error tracebacks.
//!
//! # Examples
//!
//! ```ignore
//! use anyhow;
//! use std::sync::Arc;
//! use hpotter::docker;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let docker_client = Arc::new(docker::connect()?);
//!     let has_image = docker::image_is_available(&docker_client, "postgres:17.6").await?;
//!     Ok(())
//! }
//! ```
use crate::config;
use anyhow::Result;
use bollard::Docker;
use bollard::models::{ImageSummary, VolumeCreateRequest};
use bollard::plugin::{
    ContainerCreateBody, EndpointSettings, HostConfig, NetworkCreateRequest, NetworkingConfig,
};
use bollard::query_parameters::{
    CreateContainerOptions, CreateImageOptions, InspectContainerOptions, ListImagesOptionsBuilder,
    ListVolumesOptions, LogsOptionsBuilder, RemoveVolumeOptions,
};
use bollard::query_parameters::{ListContainersOptionsBuilder, ListNetworksOptionsBuilder};
use futures_util::StreamExt;
use futures_util::future::join_all;
use futures_util::stream::TryStreamExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Connects to the local docker server by wrapping the bollard
/// docker `connect_with_socket_defaults` and returning and
/// `anyhow` error type instead of a `bollard` error type.
pub fn connect() -> Result<Docker> {
    Ok(Docker::connect_with_socket_defaults()?)
}

/// Checks if a given container image tag is available using the given
/// docker server client.
///
/// # Arguments
///
/// * `docker`: the docker server client
/// * `image`: the container image tag to search for
pub async fn image_is_available(docker: &Docker, image: &str) -> Result<bool> {
    Ok(get_images(docker)
        .await
        .iter()
        .flat_map(|i| i.iter())
        .any(|tag| tag == image))
}

/// Calls the given docker server client to list all images available
/// and returns a Vec<String> of the image tags available on the server.
///
/// # Arguments
///
/// * `docker`: the docker server client
async fn get_images(docker: &Docker) -> Result<Vec<String>> {
    let options = ListImagesOptionsBuilder::default().all(true).build();
    let images = docker.list_images(Some(options)).await?;
    Ok(extract_image_tags(&images))
}

/// Extracts the image tags given a slice of `bollard::models::ImageSummary`
/// and returns a Vec<String> containing the image tags.
///
/// # Arguments
///
/// * `images`: a slice of `bollard_stubs::models::ImageSummary`
fn extract_image_tags(images: &[ImageSummary]) -> Vec<String> {
    images.iter().flat_map(|i| i.repo_tags.clone()).collect()
}

/// Pulls a given container image using the given docker server client.
///
/// # Arguments
///
/// * `docker`: the docker server client
/// * `image`: the container image tag to pull
///
/// # Examples
///
/// ```ignore
/// use anyhow;
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let result = docker::pull_image(&docker_client, "hello-world:latest").await;
///     assert!(result.is_ok());
///     Ok(())
/// }
/// ```
pub async fn pull_image(docker: &Docker, image: &str) -> Result<()> {
    let mut stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: Some(String::from(image)),
            ..Default::default()
        }),
        None,
        None,
    );

    while let Some(event) = stream.next().await {
        event?;
    }

    Ok(())
}

/// A wrapper around `image_is_available` and `pull_image` that ensures
/// the given container image is present on the docker server
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `image`: the image to ensure is present
///
/// # Examples
///
/// ```
/// use anyhow;
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let result = docker::ensure_image(&docker_client, "hello-world:latest").await;
///     assert!(result.is_ok());
///     Ok(())
/// }
/// ```
pub async fn ensure_image(docker_client: &Docker, image: &str) -> Result<()> {
    let has_img = image_is_available(&docker_client, &image).await?;
    if !has_img {
        pull_image(&docker_client, &image).await?;
    }
    Ok(())
}

/// Iterates over `config` and ensures that all required container images
/// for the honeypot are pulled.
///
/// # Arguments
///
/// * `config`: the deserialized honeypot configuration
/// * `docker_client`: the docker server client
///
/// # Examples
///
/// ```ignore
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let config = config::load_config("config.yml")?;
///     let docker_client = Arc::new(docker::connect()?);
///     docker::download_images(&config, &docker_client).await?;
///     Ok(())
/// }
/// ```
pub async fn download_images(config: &config::Config, docker_client: &Arc<Docker>) -> Result<()> {
    let mut handles = vec![];

    let docker_client_clone = Arc::clone(&docker_client);
    let image = config.database.image.clone();
    handles.push(tokio::spawn(async move {
        ensure_image(&docker_client_clone, &image).await
    }));

    for svc in &config.services {
        let docker_client_clone = Arc::clone(&docker_client);
        let image = svc.image.clone();
        handles.push(tokio::spawn(async move {
            ensure_image(&docker_client_clone, &image).await
        }));
    }

    for result in join_all(handles).await {
        result??;
    }

    Ok(())
}

/// Ensures that the docker network resposible for hosting the database
/// container exists. Checks if the network exists and if it does not,
/// will create it.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `name`: the name of the docker network to search for
///
/// # Examples
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let network_name = "example-network";
///     let docker_client = Arc::new(docker::connect()?);
///
///     let before_networks = docker::get_network_names(&docker_client).await.unwrap();
///     assert!(!before_networks.contains(&String::from(network_name)));
///
///     let result = docker::ensure_db_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let after_networks = docker::get_network_names(&docker_client).await.unwrap();
///     assert!(after_networks.contains(&String::from(network_name)));
///
///     Ok(())
/// }
/// ```
pub async fn ensure_db_network(docker_client: &Docker, name: &str) -> Result<String> {
    let network_id = get_network_id(&docker_client, name).await?;

    if network_id.is_empty() {
        return create_network(&docker_client, name).await;
    }

    Ok(network_id)
}

/// Lists the docker networks and returns a `Vec<String>` of the found
/// network names or an error if the docker API call failed.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let network_name = "example-network";
///     let docker_client = Arc::new(docker::connect()?);
///
///     let result = docker::ensure_db_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let after_networks = docker::get_network_names(&docker_client).await.unwrap();
///     assert!(after_networks.contains(&String::from(network_name)));
///
///     Ok(())
/// }
/// ```
pub async fn get_network_names(docker_client: &Docker) -> Result<Vec<String>> {
    let options = ListNetworksOptionsBuilder::default().build();
    let networks = docker_client.list_networks(Some(options)).await?;
    Ok(networks.iter().filter_map(|n| n.name.clone()).collect())
}

/// Searches the docker networks filtering for `name` and returns
/// the corresponding docker network id or an error if not found.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `name`: the name of the docker network to search for
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let network_name = "example-network";
///     let docker_client = Arc::new(docker::connect()?);
///
///     let result = docker::ensure_db_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let network_id = docker::get_network_id(&docker_client, network_name)
///         .await.unwrap();
///     assert_ne!(network_id, "");
///
///     Ok(())
/// }
/// ```
pub async fn get_network_id(docker_client: &Docker, name: &str) -> Result<String> {
    let mut filters = HashMap::new();
    filters.insert("name", vec![format!("{name}")]);

    let options = ListNetworksOptionsBuilder::default()
        .filters(&filters)
        .build();

    let networks = docker_client.list_networks(Some(options)).await?;
    Ok(networks.iter().filter_map(|n| n.id.clone()).collect())
}

/// Creates the docker network statically named with `name`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `name`: the name of the docker network to create
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let network_name = "example-network";
///     let docker_client = Arc::new(docker::connect()?);
///
///     // ensure network isn't lingering from previous failed tests
///     let _ = docker::delete_network(&docker_client, network_name).await;
///
///     let result = docker::create_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let after_networks = docker::get_network_names(&docker_client).await.unwrap();
///     assert!(after_networks.contains(&String::from(network_name)));
///
///     let _ = docker::delete_network(&docker_client, network_name).await;
///
///     Ok(())
/// }
/// ```
pub async fn create_network(docker_client: &Docker, name: &str) -> Result<String> {
    let network_req = NetworkCreateRequest {
        name: String::from(name),
        driver: Some(String::from("bridge")),
        ..Default::default()
    };

    let resp = docker_client.create_network(network_req).await?;

    Ok(resp.id)
}

/// Deletes the docker network named with `name`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `name`: the name of the docker network to delete
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker};
///
/// async fn example() -> anyhow::Result<()> {
///     let network_name = "example-network";
///     let docker_client = Arc::new(docker::connect()?);
///
///     let result = docker::ensure_db_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let result = docker::delete_network(&docker_client, network_name).await;
///     assert!(result.is_ok());
///
///     let after_networks = docker::get_network_names(&docker_client).await.unwrap();
///     assert!(!after_networks.contains(&String::from(network_name)));
///
///     Ok(())
/// }
/// ```
pub async fn delete_network(docker_client: &Docker, name: &str) -> Result<()> {
    let network_id = get_network_id(docker_client, name).await?;
    if network_id != "" {
        docker_client.remove_network(name).await?;
    }
    Ok(())
}

/// Ensures that the docker database container exists by calling
/// `get_container_id` and checking the output. Checks if the container
/// is present and stopped, then will start it. Otherwise it will create a
/// new database container using the provided `container_conf`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `container_conf`: the container configuration struct
///
/// # Examples
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker, db};
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let db_network_id = docker::create_network(&docker_client, "hpotter")
///         .await?;
///
///     let container_conf = docker::HpotterContainerConfig {
///         name: String::from("hpotter-database"),
///         image: String::from("hello-world:latest"),
///         host_port: 5432,
///         container_port: 5432,
///         env: Some(vec![
///             String::from("POSTGRES_DB=hpotter"),
///             String::from(format!("POSTGRES_USER={}", "")),
///             String::from(format!("POSTGRES_PASSWORD={}", "")),
///         ]),
///         network_id: Some(db_network_id),
///         cmd: Some(vec![String::from("/hello")]),
///         volumes: Some(vec![String::from("hello-world-data")]),
///     };
///
///     let result = docker::ensure_db_container(&docker_client, &container_conf).await;
///     assert!(result.is_ok());
///
///     let container_id = docker::get_container_id(&docker_client, &container_conf.name).await?;
///     assert_ne!(container_id, "");
///
///     let _ = docker::delete_network(&docker_client, "hpotter").await?;
///
///     Ok(())
/// }
/// ```
pub async fn ensure_db_container(
    docker_client: &Docker,
    container_conf: &HpotterContainerConfig,
) -> Result<String> {
    let existing_container_id = match get_container_id(docker_client, &container_conf.name).await {
        Ok(id) if !id.is_empty() => id,
        Ok(_) | Err(_) => {
            return create_container(docker_client, container_conf).await;
        }
    };

    Ok(existing_container_id)
}

/// Searches for a container with a name matching `name` and returns `true`
/// if found, otherwise returns `false`. An error will be returned if
/// encountered when calling the docker API.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `name`: the name of the container to search for
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let container_name = "hello-world";
///     let docker_client = Arc::new(docker::connect()?);
///
///     let result = docker::get_container_id(&docker_client, container_name).await;
///     // result is true if container exists, false otherwise
///
///     Ok(())
/// }
/// ```
pub async fn get_container_id(docker_client: &Docker, name: &str) -> Result<String> {
    let mut filters = HashMap::new();
    filters.insert("name", vec![format!("{name}")]);

    let options = ListContainersOptionsBuilder::default()
        .all(true)
        .filters(&filters)
        .build();

    let containers = docker_client.list_containers(Some(options)).await?;
    Ok(containers.iter().filter_map(|c| c.id.clone()).collect())
}

#[derive(Debug, PartialEq)]
pub struct HpotterContainerConfig {
    pub name: String,
    pub image: String,
    pub host_port: u16,
    pub container_port: u16,
    pub env: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub network_id: Option<String>,
    pub volumes: Option<Vec<String>>,
}

/// Builds the network config for the container and returns the
/// `NetworkConfig` to be used when creating the container using the
/// bollard API.
///
/// # Arguments
///
/// * `container_conf`: the HpotterContainerConfig struct with the container configuration
async fn get_network_config(container_conf: &HpotterContainerConfig) -> NetworkingConfig {
    if container_conf.network_id.is_some() {
        let ep_settings = EndpointSettings {
            network_id: container_conf.network_id.clone(),
            ..Default::default()
        };

        let mut ep_config = HashMap::new();
        ep_config.insert(String::from("network-config"), ep_settings);

        NetworkingConfig {
            endpoints_config: Some(ep_config),
        }
    } else {
        NetworkingConfig {
            ..Default::default()
        }
    }
}

/// Builds the host config for the container and returns the `HostConfig`
/// to be used when creating the container using the bollard API.
///
/// # Arguments
///
/// * `container_conf`: the HpotterContainerConfig struct with the container configuration
async fn get_host_config(container_conf: &HpotterContainerConfig) -> HostConfig {
    HostConfig {
        port_bindings: Some(HashMap::from([(
            format!("{}/tcp", container_conf.container_port),
            Some(vec![bollard::models::PortBinding {
                host_ip: Some(String::from("127.0.0.1")),
                host_port: Some(container_conf.host_port.to_string()),
            }]),
        )])),
        network_mode: Some(String::from("bridge")),
        ..Default::default()
    }
}

/// Creates a new container using the given docker server client and
/// `HpotterContainerConfig`. This function will only create the container
/// and will not start it. The associated container id is returned, or an
/// error if encountered when calling the docker API.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `container_conf`: the HpotterContainerConfig struct with the container configuration
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker, db};
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///
///     let container_conf = docker::HpotterContainerConfig {
///         name: String::from("hello-world"),
///         image: String::from("hello-world:latest"),
///         host_port: 8080,
///         container_port: 8080,
///         env: None,
///         network_id: None,
///         cmd: Some(vec![String::from("/hello")]),
///         volumes: None,
///     };
///
///     // the response will be the container's associated id
///     let container_id = docker::create_container(&docker_client, &container_conf).await?;
///     assert_ne!(container_id, "");
///
///     let container_id = docker::get_container_id(&docker_client, &container_conf.name).await?;
///     assert_ne!(container_id, "");
///
///     let _ = docker::delete_container(&docker_client, &container_id).await;
///
///     Ok(())
/// }
/// ```
pub async fn create_container(
    docker_client: &Docker,
    container_conf: &HpotterContainerConfig,
) -> Result<String> {
    let options = CreateContainerOptions {
        name: Some(container_conf.name.clone()),
        ..Default::default()
    };

    let network_config = get_network_config(container_conf).await;
    let host_config = get_host_config(container_conf).await;

    let body = ContainerCreateBody {
        image: Some(container_conf.image.clone()),
        hostname: Some(String::from("diagon-alley")),
        domainname: Some(String::from("hogwarts")),
        networking_config: Some(network_config),
        host_config: Some(host_config),
        env: container_conf.env.clone(),
        cmd: container_conf.cmd.clone(),
        volumes: container_conf.volumes.clone(),
        ..Default::default()
    };

    let resp = docker_client.create_container(Some(options), body).await?;

    Ok(resp.id)
}

/// Attempts to stop and delete the container image specified by the given
/// container `id`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `id`: the container id
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker, db};
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///
///     let container_conf = docker::HpotterContainerConfig {
///         name: String::from("hello-world"),
///         image: String::from("hello-world:latest"),
///         host_port: 8080,
///         container_port: 8080,
///         env: None,
///         network_id: None,
///         cmd: Some(vec![String::from("/hello")]),
///         volumes: None,
///     };
///
///     // the response will be the container's associated id
///     let container_id = docker::create_container(&docker_client, &container_conf).await?;
///     assert_ne!(container_id, "");
///
///     let container_id = docker::get_container_id(&docker_client, &container_conf.name).await?;
///     assert_ne!(container_id, "");
///
///     let result = docker::delete_container(&docker_client, &container_id).await;
///     assert!(result.is_ok());
///
///     Ok(())
/// }
/// ```
pub async fn delete_container(docker_client: &Docker, id: &str) -> Result<()> {
    stop_container(docker_client, id).await?;
    docker_client.remove_container(id, None).await?;
    Ok(())
}

/// Starts the container image specified by the given container `id`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `id`: the container id or container name
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::{config, docker, db};
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///
///     let container_conf = docker::HpotterContainerConfig {
///         name: String::from("test_container_start_succeeds"),
///         image: String::from("hello-world:latest"),
///         host_port: 5432,
///         container_port: 5432,
///         env: None,
///         network_id: None,
///         cmd: Some(vec![String::from("/hello")]),
///         volumes: None,
///     };
///
///     let container_id = docker::create_container(&docker_client, &container_conf).await?;
///     assert_ne!(container_id, "");
///
///     let result = docker::start_container(&docker_client, &container_id).await;
///     assert!(result.is_ok());
///
///     let del_result = docker::delete_container(&docker_client, &container_id).await;
///     assert!(del_result.is_ok());
///
///     Ok(())
/// }
/// ```
pub async fn start_container(docker_client: &Docker, id: &str) -> Result<()> {
    docker_client.start_container(id, None).await?;
    Ok(())
}

/// Stops the container image specified by the given container `id`.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `id`: the container id
async fn stop_container(docker_client: &Docker, id: &str) -> Result<()> {
    docker_client.stop_container(id, None).await?;
    Ok(())
}

/// Calls list volumes using the given docker server client.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `volume_name`: the name of the volume to create
async fn volume_exists(docker_client: &Docker, volume_name: &str) -> Result<bool> {
    let mut filters = HashMap::new();
    filters.insert(String::from("name"), vec![format!("{volume_name}")]);

    let options = ListVolumesOptions {
        filters: Some(filters),
        ..Default::default()
    };

    let response = docker_client.list_volumes(Some(options)).await?;

    match response.volumes {
        Some(volumes) => Ok(!volumes.is_empty()),
        None => Ok(false),
    }
}

/// Creates a volume using the given volume name and docker server client.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `volume_name`: the name of the volume to create
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let db_volume_name = "hpotter-data-create-volume-example";
///
///     let result = docker::create_volume(&docker_client, &db_volume_name)
///         .await;
///     assert!(result.is_ok());
///
///     let _ = docker::delete_volume(&docker_client, &db_volume_name)
///         .await;
///
///     Ok(())
/// }
/// ```
pub async fn create_volume(docker_client: &Docker, volume_name: &str) -> Result<()> {
    let volume_req = VolumeCreateRequest {
        name: Some(String::from(volume_name)),
        ..Default::default()
    };

    let _volume = docker_client.create_volume(volume_req).await?;
    Ok(())
}

/// Checks if the given volume exists and creates it if it doesn't.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `volume_name`: the name of the volume to create
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let db_volume_name = "hpotter-data-ensure-volume-example";
///
///     let result = docker::ensure_db_volume(&docker_client, &db_volume_name)
///         .await;
///     assert!(result.is_ok());
///
///     let _ = docker::delete_volume(&docker_client, &db_volume_name)
///         .await;
///
///     Ok(())
/// }
/// ```
pub async fn ensure_db_volume(docker_client: &Docker, volume_name: &str) -> Result<()> {
    if !volume_exists(docker_client, volume_name).await? {
        create_volume(docker_client, volume_name).await?;
    }
    Ok(())
}

/// Deletes the volume given the volume name and a docker server client.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `volume_name`: the name of the volume to create
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let db_volume_name = "hpotter-data-volume-example";
///
///     let result = docker::delete_volume(&docker_client, &db_volume_name)
///         .await;
///     assert!(result.is_ok());
///
///     Ok(())
/// }
/// ```
pub async fn delete_volume(docker_client: &Docker, volume_name: &str) -> Result<()> {
    if volume_exists(docker_client, volume_name).await? {
        docker_client
            .remove_volume(volume_name, Some(RemoveVolumeOptions::default()))
            .await?;
    }
    Ok(())
}

/// Returns the logs for the container associated with `id` using the given
/// docker server client.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `id`: the id or name of the container to get logs for
///
/// # Examples
///
/// ```ignore
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let container_id = "example-container-id-or-name";
///
///     let container_logs = docker::get_container_logs(&docker_client, &container_id)
///         .await?;
///     assert_ne(container_logs, "");
///
///     Ok(())
/// }
/// ```
pub async fn get_container_logs(docker_client: &Docker, id: &str) -> Result<String> {
    let options = LogsOptionsBuilder::default()
        .stdout(true)
        .stderr(true)
        .tail("all")
        .build();

    let logs: Vec<String> = docker_client
        .logs(id, Some(options))
        .map_ok(|log| log.to_string())
        .try_collect()
        .await?;

    Ok(logs.join(""))
}

/// Returns the IP addresss for the container associated with `id` using the
/// given docker server client.
///
/// # Arguments
///
/// * `docker_client`: the docker server client
/// * `id`: the id or name of the container to get logs for
///
/// # Examples
///
/// ```ignore
/// use std::sync::Arc;
/// use hpotter::docker;
///
/// async fn example() -> anyhow::Result<()> {
///     let docker_client = Arc::new(docker::connect()?);
///     let container_id = "example-container-id-or-name";
///
///     let container_ip = docker::get_container_ip(&docker_client, &container_id)
///         .await?;
///
///     assert_ne(container_ip, "");
///
///     Ok(())
/// }
/// ```
pub async fn get_container_ip(docker: &Docker, id: &str) -> Result<String> {
    let options = InspectContainerOptions::default();
    let inspect_response = docker.inspect_container(id, Some(options)).await?;

    let ip = inspect_response
        .network_settings
        .and_then(|settings| settings.networks)
        .and_then(|networks| {
            networks
                .into_iter()
                .next()
                .and_then(|(_, network)| network.ip_address)
        })
        .unwrap_or_default();

    Ok(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_tags_found() {
        let images = vec![bollard::models::ImageSummary {
            id: String::from("1"),
            repo_tags: vec![String::from("foo/bar:v1.0.0")],
            ..Default::default()
        }];
        let available_images = extract_image_tags(&images);
        assert_eq!(available_images.len(), 1);
        assert_eq!(&available_images[0], "foo/bar:v1.0.0")
    }

    #[test]
    fn test_extract_image_tags_not_found() {
        let images = vec![bollard::models::ImageSummary {
            id: String::from("1"),
            ..Default::default()
        }];
        let available_images = extract_image_tags(&images);
        assert_eq!(available_images.len(), 0);
    }

    #[test]
    fn test_extract_image_tags_empty() {
        let images = vec![];
        let available_images = extract_image_tags(&images);
        assert!(available_images.is_empty());
    }

    // TODO: test the remaining private functions
}
