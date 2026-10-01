# alerts

A library for handling and raising critical alerts.

## Alert fields

| Field       | Type                      | Optional? | Meaning                                                                                                                                                                                                         |
|-------------|---------------------------|-----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `source`    | `String`                  | No        | The service raising the alert, e.g. `admin/lookup-sync`.                                                                                                                                                        |
| `dedup_key` | `String`                  | No        | A stable name for the problem, e.g. `sync-failed`. Use the same key every time the same problem happens. The intention is that `(source, dedup_key)` can be used to deduplicate issues for debouncing purposes. |
| `summary`   | `String`                  | No        | One succinct line saying what went wrong.                                                                                                                                                                       |
| `detail`    | `Option<String>`          | Yes       | Supplementary longer text, e.g.: the full chain of errors.                                                                                                                                                      |
| `extra`     | `HashMap<String, String>` | Yes       | Supplementary key/value context, e.g. `group_id`.                                                                                                                                                               |

## Publishing an alert

```rust
use alerts::AlertPublisher;

// e.g. "projects/kcsu-admin/topics/alerts"
let topic = common::require_env("ALERTS_TOPIC")?;
let alerts = AlertPublisher::new(&topic, "admin/lookup-sync").await?;

alerts
    .alert("sync-failed", "Lookup sync failed")
    .detail("lookup request for group 105777 failed: timed out")
    .extra("group_id", "105777")
    .send()
    .await?;
```

## Handling an alert

There are currently two alert handlers:

| Handler      | Description                                                                        |
|--------------|------------------------------------------------------------------------------------|
| `GoogleChat` | Posts the alert to a Google Chat space as a formatted card.                        |
| `Firestore`  | Stores the alert as a document in the `alerts` collection, with a TTL of 180 days. |

Delivery to the handlers has at most once semantics.

To add another handler, add a variant to `AlertHandler` in `src/consumer.rs`.
