# kcsu/admin

Backend services and automations for KCSU.

## Services

| Service         | Description                                                                                                            |
|-----------------|------------------------------------------------------------------------------------------------------------------------|
| `alerting`      | Receive alerts from GCP pub/sub, and dispatch it to downstream handlers. See [crates/alerts](crates/alerts/README.md). |
| `lookup-sync`   | Fetches group membership from the Cambridge UIS Lookup API on a regular basis.                                         |
| `uptime`        | Checks KCSU's sites every minute and alerts when one is down. See [crates/uptime](crates/uptime/README.md).            |
| `serve`         | Serves the public API at `/api/public/*`.                                                                              |
| `admin-console` | Serves the admin console site and its private API, behind Google sign-in.                                              |

## Getting started

```sh
git clone --recurse-submodules https://github.com/KCSU/admin.git
# or, in an existing clone:
git submodule update --init

cargo build
```
