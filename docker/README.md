# Building blanc OS using Docker
You just need `git`, `make`, and `docker`.
It is better to use a non-privileged user to run the `docker` command, which is usually achieved by adding the user to the `docker` group.

## Run the container to build blanc os
You can build the docker image using `make docker_build` and run it using `make docker_run`.

## Run the container interactively
You can use the `make` target `docker_interactive` to get a shell in the container.

## Clear the toolchain caches (Cargo & Rustup)
To clean the docker volumes used by the toolchain, you just need to run `make docker_clean`.

