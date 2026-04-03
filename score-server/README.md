# Neon Invaders Score Server

Minimal authenticated score backend for Neon Invaders.

## What It Does

- email-based magic-link login
- short-lived access tokens
- long-lived refresh tokens
- persistent local player / score storage
- leaderboard endpoint
- score submission endpoint

## Important Security Note

This authenticates players, but it does not make submitted scores cheat-proof. A desktop game client is user-controlled. If you need strong anti-cheat, you need replay verification or a more authoritative backend model.

## Run

```bash
cd score-server
cargo run
```

Defaults:

- bind: `127.0.0.1:8787`
- base URL: `http://127.0.0.1:8787`
- data dir: `./score-server/data`

Optional environment variables:

```bash
SCORE_SERVER_BIND=0.0.0.0:8787
SCORE_SERVER_BASE_URL=https://scores.example.com
SCORE_SERVER_DATA_DIR=/var/lib/neon-invaders-score-server
```

## API

### Request magic link

```bash
curl -X POST http://127.0.0.1:8787/auth/request-link \
  -H 'content-type: application/json' \
  -d '{"email":"pilot@example.com"}'
```

For development, the generated login link is appended to:

```text
score-server/data/magic-links.log
```

### Verify magic link token

```bash
curl -X POST http://127.0.0.1:8787/auth/verify \
  -H 'content-type: application/json' \
  -d '{"token":"..."}'
```

### Refresh session

```bash
curl -X POST http://127.0.0.1:8787/auth/refresh \
  -H 'content-type: application/json' \
  -d '{"refresh_token":"..."}'
```

### Submit score

```bash
curl -X POST http://127.0.0.1:8787/scores \
  -H 'content-type: application/json' \
  -H 'authorization: Bearer ACCESS_TOKEN' \
  -d '{"score":12345,"wave":7,"app_version":"0.1.0"}'
```

### Read leaderboard

```bash
curl 'http://127.0.0.1:8787/leaderboard?limit=20'
```
