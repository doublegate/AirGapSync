# Configuration Schema

Define sync policies in a JSON or TOML file, e.g.:

```toml
[source]
path = "/Users/username/Documents"

[device]
id = "USB001"
mount_point = "/Volumes/USB001"

[policy]
retain_snapshots = 3
gc_interval_days = 7
```
