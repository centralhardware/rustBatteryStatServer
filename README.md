# BatteryStatServer

Collects MacBook battery samples into ClickHouse. The macOS agent that sends them
lives in [`agent/`](agent/README.md).

## API

- `POST /api/battery/health`: one sample as JSON (camelCase fields of
  [`BatteryHealth`](src/model.rs)), answered with `201`. `dateTime` is the sampling
  time in unix seconds; if it's missing, the time of arrival is used.
  When `API_TOKEN` is set, requests must carry `Authorization: Bearer <API_TOKEN>`.
- `GET /ping`: `200` if ClickHouse answers, `503` if not. The Docker health check uses it.

## Configuration

| env | default |
|---|---|
| `CH_URL` | `http://localhost:8123` |
| `CH_DB` | `default` |
| `CH_USER` / `CH_PASSWORD` | `default` / empty |
| `API_TOKEN` | unset (no auth, logged as a warning) |
| `PORT` | `8080` |

## Schema

Table `battery_health` (MergeTree, `ORDER BY (device_id, date_time)`). Its
history is in [`migrations/`](migrations/). New migrations are applied by hand
before deploying the code that needs them.

## Deploy

Pushing to `main` builds `ghcr.io/centralhardware/rustbatterystatserver` and
triggers the Coolify deploy hook. Changes that touch only `agent/`, `migrations/`
or docs don't trigger a deploy.
