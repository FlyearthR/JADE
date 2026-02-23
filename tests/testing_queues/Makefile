.PHONY: test
test:
	cargo run --bin receiver &
	docker run --rm --ipc=host -v $(pwd):/app -w /app rust:latest cargo run --bin sender
	docker run --rm -d --ipc=host -v $(pwd):/app -w /app rust:latest cargo run --bin receiver
	cargo run --bin sender