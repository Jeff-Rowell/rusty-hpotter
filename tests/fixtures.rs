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

pub const EXPECTED_CONTAINER_LOGS: &str = r#"
Hello from Docker!
This message shows that your installation appears to be working correctly.

To generate this message, Docker took the following steps:
 1. The Docker client contacted the Docker daemon.
 2. The Docker daemon pulled the "hello-world" image from the Docker Hub.
    (amd64)
 3. The Docker daemon created a new container from that image which runs the
    executable that produces the output you are currently reading.
 4. The Docker daemon streamed that output to the Docker client, which sent it
    to your terminal.

To try something more ambitious, you can run an Ubuntu container with:
 $ docker run -it ubuntu bash

Share images, automate workflows, and more with a free Docker ID:
 https://hub.docker.com/

For more examples and ideas, visit:
 https://docs.docker.com/get-started/

"#;
