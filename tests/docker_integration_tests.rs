mod fixtures;
use bollard::Docker;
use fixtures::{EXPECTED_CONTAINER_LOGS, TEST_BAD_CONFIG, TEST_GOOD_CONFIG};
use hpotter::config::Config;
use hpotter::docker::{
    HpotterContainerConfig, create_container, create_network, create_volume, delete_container,
    delete_network, delete_volume, download_images, ensure_db_container, ensure_db_network,
    ensure_db_volume, get_container_id, get_container_logs, get_network_id, get_network_names,
    image_is_available, pull_image, start_container,
};
use std::sync::Arc;
use testcontainers::GenericImage;
use testcontainers::runners::AsyncRunner;

#[tokio::test]
async fn test_is_image_available_returns_true() {
    let _container = GenericImage::new("hello-world", "latest")
        .start()
        .await
        .unwrap();

    let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    assert!(
        image_is_available(&docker, "hello-world:latest")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_is_image_not_available_returns_false() {
    let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    assert!(
        !image_is_available(&docker, "this-image-does-not-exist:latest")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_pull_image_succeeds() {
    let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let result = pull_image(&docker, "hello-world:latest").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_pull_image_fails() {
    let docker = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let result = pull_image(&docker, "this-image-does-not-exist:latest").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_download_images_succeeds() {
    let config: Config = serde_norway::from_str(TEST_GOOD_CONFIG).unwrap();
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let result = download_images(&config, docker_client).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_download_images_fails() -> () {
    let config: Config = serde_norway::from_str(TEST_BAD_CONFIG).unwrap();
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let result = download_images(&config, docker_client).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_network_names_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name_1 = "testing-get-network-1";
    let network_name_2 = "testing-get-network-2";

    let result = ensure_db_network(&docker_client, network_name_1).await;
    assert!(result.is_ok());

    let result = ensure_db_network(&docker_client, network_name_2).await;
    assert!(result.is_ok());

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(
        after_networks.contains(&String::from(network_name_1))
            && after_networks.contains(&String::from(network_name_2))
    );

    let _ = delete_network(&docker_client, network_name_1).await;
    let _ = delete_network(&docker_client, network_name_2).await;
}

#[tokio::test]
async fn test_create_network_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "testing-create-network";

    // ensure network isn't lingering from previous failed tests
    let _ = delete_network(&docker_client, network_name).await;

    let result = create_network(&docker_client, network_name).await.unwrap();
    assert_ne!(result, "");

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(after_networks.contains(&String::from(network_name)));

    let _ = delete_network(&docker_client, network_name).await;
}

#[tokio::test]
async fn test_delete_network_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "testing-delete-network";

    // ensure network isn't lingering from previous failed tests
    let _ = delete_network(&docker_client, network_name).await;

    let result = ensure_db_network(&docker_client, network_name).await;
    assert!(result.is_ok());

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(after_networks.contains(&String::from(network_name)));

    let delete_result = delete_network(&docker_client, network_name).await;
    assert!(delete_result.is_ok());

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(!after_networks.contains(&String::from(network_name)));
}

#[tokio::test]
async fn test_ensure_db_network_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "testing-ensure-db-network-creation";

    // ensure network isn't lingering from previous failed tests
    let _ = delete_network(&docker_client, network_name).await;

    let before_networks = get_network_names(&docker_client).await.unwrap();
    assert!(!before_networks.contains(&String::from(network_name)));

    let result = ensure_db_network(&docker_client, network_name).await;
    assert!(result.is_ok());

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(after_networks.contains(&String::from(network_name)));

    let _ = delete_network(&docker_client, network_name).await;
}

#[tokio::test]
async fn test_ensure_db_network_succeeds_when_exists() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "testing-ensure-db-network-creation-exists";

    // ensure network isn't lingering from previous failed tests
    let _ = delete_network(&docker_client, network_name).await;

    let _ = create_network(&docker_client, network_name).await;

    let result = ensure_db_network(&docker_client, network_name).await;
    assert!(result.is_ok());

    let after_networks = get_network_names(&docker_client).await.unwrap();
    assert!(after_networks.contains(&String::from(network_name)));

    let _ = delete_network(&docker_client, network_name).await;
}

#[tokio::test]
async fn test_get_network_id_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "test-network-id-succeeds";

    let result = ensure_db_network(&docker_client, network_name).await;
    assert!(result.is_ok());

    let network_id = get_network_id(&docker_client, network_name).await.unwrap();
    assert_ne!(network_id, "");
}

#[tokio::test]
async fn test_get_network_id_fails() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "this-never-exists-and-doesnt-have-an-id";

    let network_id = get_network_id(&docker_client, network_name).await.unwrap();
    assert_eq!(network_id, "");
}

#[tokio::test]
async fn test_create_container_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let container_conf = HpotterContainerConfig {
        name: String::from("hello-world-container-succeeds"),
        image: String::from("hello-world:latest"),
        host_port: 8080,
        container_port: 8080,
        env: None,
        cmd: Some(vec![String::from("/hello")]),
        volumes: None,
        network_id: None,
    };

    // the response will be the container's associated id
    let container_id = create_container(&docker_client, &container_conf)
        .await
        .unwrap();
    assert_ne!(container_id, "");

    let container_id = get_container_id(&docker_client, &container_conf.name)
        .await
        .unwrap();
    assert_ne!(container_id, "");

    let del_res = delete_container(&docker_client, &container_id).await;
    assert!(del_res.is_ok());
}

#[tokio::test]
async fn test_get_container_id_fails() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let container_name = "a-non-existent-container-name";
    let container_id = get_container_id(&docker_client, &container_name)
        .await
        .unwrap();

    assert_eq!(container_id, "")
}

#[tokio::test]
async fn test_delete_container_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let container_conf = HpotterContainerConfig {
        name: String::from("hello-world-delete-success"),
        image: String::from("hello-world:latest"),
        host_port: 8080,
        container_port: 8080,
        env: None,
        cmd: Some(vec![String::from("/hello")]),
        volumes: None,
        network_id: None,
    };

    // the response will be the container's associated id
    let container_id = create_container(&docker_client, &container_conf)
        .await
        .unwrap();
    assert_ne!(container_id, "");

    let result = delete_container(&docker_client, &container_id).await;
    assert!(result.is_ok())
}

#[tokio::test]
async fn test_delete_container_fails() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let container_id = "a-non-existent-container-id";
    let result = delete_container(&docker_client, &container_id).await;
    assert!(result.is_err())
}

#[tokio::test]
async fn test_ensure_db_container_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "ensure_db_container_network_succeeds";
    let db_network_id = ensure_db_network(&docker_client, &network_name)
        .await
        .unwrap();

    let container_conf = HpotterContainerConfig {
        name: String::from("ensure-db-container-succeeds"),
        image: String::from("hello-world:latest"),
        host_port: 5432,
        container_port: 5432,
        env: Some(vec![
            String::from("POSTGRES_DB=hpotter"),
            String::from(format!("POSTGRES_USER={}", "")),
            String::from(format!("POSTGRES_PASSWORD={}", "")),
        ]),
        network_id: Some(db_network_id),
        cmd: Some(vec![String::from("/hello")]),
        volumes: Some(vec![String::from("hello-world-data")]),
    };

    let container_id = ensure_db_container(&docker_client, &container_conf)
        .await
        .unwrap();

    assert_ne!(container_id, "");

    let result = delete_container(&docker_client, &container_id).await;
    assert!(result.is_ok());

    let _ = delete_network(&docker_client, &network_name).await.unwrap();
}

#[tokio::test]
async fn test_ensure_db_container_exists() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let network_name = "ensure_db_container_network_exists";
    let db_network_id = ensure_db_network(&docker_client, &network_name)
        .await
        .unwrap();

    let container_conf = HpotterContainerConfig {
        name: String::from("ensure-db-container-exists"),
        image: String::from("hello-world:latest"),
        host_port: 5432,
        container_port: 5432,
        env: Some(vec![
            String::from("POSTGRES_DB=hpotter"),
            String::from(format!("POSTGRES_USER={}", "")),
            String::from(format!("POSTGRES_PASSWORD={}", "")),
        ]),
        network_id: Some(db_network_id),
        cmd: Some(vec![String::from("/hello")]),
        volumes: Some(vec![String::from("hello-world-data")]),
    };

    let actual_container_id = create_container(&docker_client, &container_conf)
        .await
        .unwrap();
    assert_ne!(actual_container_id, "");

    let container_id = ensure_db_container(&docker_client, &container_conf)
        .await
        .unwrap();

    assert_eq!(container_id, actual_container_id);

    let result = delete_container(&docker_client, &container_id).await;
    assert!(result.is_ok());

    let _ = delete_network(&docker_client, &network_name).await.unwrap();
}

#[tokio::test]
async fn test_container_start_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());

    let container_conf = HpotterContainerConfig {
        name: String::from("test_container_start_succeeds"),
        image: String::from("hello-world:latest"),
        host_port: 5432,
        container_port: 5432,
        env: None,
        network_id: None,
        cmd: Some(vec![String::from("/hello")]),
        volumes: None,
    };

    let container_id = create_container(&docker_client, &container_conf)
        .await
        .unwrap();
    assert_ne!(container_id, "");

    let result = start_container(&docker_client, &container_id).await;
    assert!(result.is_ok());

    let del_result = delete_container(&docker_client, &container_id).await;
    assert!(del_result.is_ok());
}

#[tokio::test]
async fn test_delete_volume_fails() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let db_volume_name = "this-volume-does-not-exist";
    let del_result = delete_volume(&docker_client, &db_volume_name).await;
    assert!(del_result.is_ok());
}

#[tokio::test]
async fn test_ensure_db_volume_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let db_volume_name = "ensure_db_volume_succeeds_volume";

    let result = ensure_db_volume(&docker_client, &db_volume_name).await;
    assert!(result.is_ok());

    let del_result = delete_volume(&docker_client, &db_volume_name).await;
    assert!(del_result.is_ok());
}

#[tokio::test]
async fn test_ensure_db_volume_exists() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let db_volume_name = "ensure_db_volume_exists_volume";

    let create_result = create_volume(&docker_client, &db_volume_name).await;
    assert!(create_result.is_ok());

    let result = ensure_db_volume(&docker_client, &db_volume_name).await;
    assert!(result.is_ok());

    let del_result = delete_volume(&docker_client, &db_volume_name).await;
    assert!(del_result.is_ok());
}

#[tokio::test]
async fn test_get_container_logs_succeeds() -> () {
    let docker_client = Arc::new(Docker::connect_with_socket_defaults().unwrap());
    let container_name = String::from("test_get_container_logs_succeeds");
    let _ = delete_container(&docker_client, &container_name).await;

    let container_conf = HpotterContainerConfig {
        name: container_name,
        image: String::from("hello-world:latest"),
        host_port: 54321,
        container_port: 54321,
        env: None,
        network_id: None,
        cmd: Some(vec![String::from("/hello")]),
        volumes: None,
    };

    let container_id = create_container(&docker_client, &container_conf)
        .await
        .unwrap();
    assert_ne!(container_id, "");

    let result = start_container(&docker_client, &container_id).await;
    assert!(result.is_ok());

    let container_logs = get_container_logs(&docker_client, &container_id)
        .await
        .unwrap();

    println!("{container_logs:#?}");
    assert_eq!(container_logs, EXPECTED_CONTAINER_LOGS);

    let del_result = delete_container(&docker_client, &container_id).await;
    assert!(del_result.is_ok());
}
