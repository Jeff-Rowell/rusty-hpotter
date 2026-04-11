# Rust Hpotter

A honey pot project to practice rust. It works in a couple simple steps:
1. Parses YAML config file that decalres what ports to listen on and what container image to use for that port
2. Starts a socket server listening on each of the `listen_port`s declared in the YAML config file
3. When a connection is received, the container is spun up in a thread
4. Two additional threads are created for that container thread, one to write requests and one to read responses from the container
5. When the client terminates or a timeout is reached, the container logs are parsed for usernames, passwords, and payloads
6. Then, the container is stopped and terminated

More docs will be added, but this is mostly just for me to practice rust.
