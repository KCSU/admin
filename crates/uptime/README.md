# uptime

A Cloud Run job, run every minute, that checks the health of KCSU's sites, stores the results for the status page, and raises an alert when an outage is detected.

## Sites

Sites are listed in `src/sites.rs`. A check passes if the site's health endpoint returns HTTP 200 with `{"status": "ok"}`.

## Alerting

| Rule          | Value                                               |
|---------------|-----------------------------------------------------|
| Outage opens  | 3 of the last 10 checks failed.                     |
| Outage closes | None of the last 10 checks failed.                  |
| Send alert    | When an outage starts, then hourly until it closes. |
