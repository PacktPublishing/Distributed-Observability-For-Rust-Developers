# Tokio Console Test

## Purpose
View `orders` async runtime activity from the host using `tokio-console`.

## Required setup
- Chapter 10 stack is running.
- `orders` has `TOKIO_CONSOLE_ENABLED=1`.
- Port `6669` is published from `orders`.
- Host has `tokio-console` installed.

## Start the stack

```bash
docker compose -p chapter_9 down
docker compose -p chapter_10 up -d --build
```

## Verify `orders`

```bash
docker ps --filter name=orders-service
docker port orders-service
nc -vz 127.0.0.1 6669
```

Expected port mapping:

```text
3003/tcp -> 0.0.0.0:3003
6669/tcp -> 0.0.0.0:6669
```

## Start Tokio Console

```bash
tokio-console http://127.0.0.1:6669 --retain-for 5m
```

Notes:
- `tokio-console` is a terminal UI, not a web page.
- Do not open `127.0.0.1:6669` in a browser.

## Generate traffic

```bash
./scripts/generate_orders_console_traffic.sh --requests 5000 --concurrency 40
```

Default generator target:

```text
http://localhost:4200/api/orders
```

## Useful filtered logs

```bash
docker logs -f orders-service 2>&1 | grep -E 'ERROR|WARN|panic|timeout|starvation|failed'
```

When Tokio Console is enabled, `orders` logs include file and line numbers.

## Troubleshooting

### Console connects but shows nothing

```bash
docker compose -p chapter_10 up -d --build orders
./scripts/generate_orders_console_traffic.sh --requests 5000 --concurrency 40
```

### Protocol mismatch
The server must use a `console-subscriber` version compatible with the installed `tokio-console` client.

Verified fix:
- `console-subscriber = 0.5.0`

### Diagnostics

```bash
tokio-console http://127.0.0.1:6669 --retain-for 5m --log trace
ls -1t /tmp/tokio-console/logs/*.log | head -n 1
```

## Verified result
- Chapter 10 stack deployed successfully.
- `orders-service` is running.
- `TOKIO_CONSOLE_ENABLED=1` is present.
- Port `6669` is reachable from host.
- Gateway traffic to `/api/orders` succeeds.
- `tokio-console` connects successfully.

## Try different filters
`tokio-console http://127.0.0.1:6669 --retain-for 5m -W self-wakes,never-yielded,lost-waker,large-future`
