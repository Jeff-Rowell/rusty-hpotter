pub const TEST_GOOD_CONFIG: &str = r#"
services:
  - name: hello-world
    num_threads: 1
    listen_address: "0.0.0.0"
    listen_port: 8080
    listen_proto: "tcp"
    image: "hello-world:latest"
    container_port: 80
    username_pattern: "test"
    password_pattern: "test"
    payload_pattern: "test"

database:
  image: "hello-world:latest"
  port: 5432
"#;

pub const TEST_BAD_CONFIG: &str = r#"
services:
  - name: hello-world
    num_threads: 1
    listen_address: "0.0.0.0"
    listen_port: 8080
    listen_proto: "tcp"
    image: "this-image-does-not-exist:latest"
    container_port: 80
    username_pattern: "test"
    password_pattern: "test"
    payload_pattern: "test"

database:
  image: "this-image-does-not-exist:latest"
  port: 5432
"#;
