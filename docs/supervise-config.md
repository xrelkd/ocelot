# Supervise Configuration

The `supervise` command uses a YAML configuration file to define managed processes. It is designed for managing complex containerized workloads with enterprise-grade features like health probes, restart policies, and dependency management.

Generate the default configuration template with:

```bash
ocelot supervise config-template
```

> **Note**: Duration fields (e.g., `terminationGracePeriod`, probe timings, `backoff`) accept human-readable strings like "30s", "1m", "1h", "2h 30m", etc. Size fields like `maxSizeBytes` accept integers (bytes) or human-readable strings like "10MB", "1GB", "512KB", "1.5GiB".

## Configuration Schema

| Field                                     | Type              | Description                                    |
| ----------------------------------------- | ----------------- | ---------------------------------------------- |
| `version`                                 | string            | Configuration version (e.g., "1.0")            |
| `processes`                               | map               | Map of process definitions                     |
| `processes.<name>.program`                | string            | Path to the executable                         |
| `processes.<name>.arguments`              | list              | Command-line arguments                         |
| `processes.<name>.environmentVariables`   | map               | Environment variables                          |
| `processes.<name>.workingDirectory`       | string            | Working directory                              |
| `processes.<name>.terminationGracePeriod` | string (duration) | Grace period before termination (e.g., "30s")  |
| `processes.<name>.readinessProbe`         | object            | Readiness probe configuration                  |
| `processes.<name>.livenessProbe`          | object            | Liveness probe configuration                   |
| `processes.<name>.restartPolicy`          | object            | Restart policy configuration                   |
| `processes.<name>.shutdownSignal`         | object            | Signal to send on shutdown                     |
| `processes.<name>.log`                    | object            | Log configuration for stdout/stderr (optional) |

## Duration Format

Duration fields accept human-readable time strings using the `humantime` format. The duration is a concatenation of time spans, where each time span consists of an integer number and a suffix.

**Supported suffixes:**

- `nsec`, `ns` – nanoseconds
- `usec`, `us`, `µs` – microseconds
- `msec`, `ms` – milliseconds
- `seconds`, `second`, `sec`, `s`
- `minutes`, `minute`, `min`, `m`
- `hours`, `hour`, `hr`, `hrs`, `h`
- `days`, `day`, `d`
- `weeks`, `week`, `wk`, `wks`, `w`
- `months`, `month`, `M` – defined as 30.44 days
- `years`, `year`, `yr`, `yrs`, `y` – defined as 365.25 days

**Examples:**

```yaml
terminationGracePeriod: 30s
terminationGracePeriod: 1m 30s
terminationGracePeriod: 2h 15m
terminationGracePeriod: 1h 30m 45s
initialDelay: 250ms
backoff: 1.5s
```

## Size Format

Size fields like `maxSizeBytes` accept either raw integers (bytes) or human-readable strings using SI/IEC prefixes.

**Supported units (case-insensitive):**

- SI (decimal) units: `KB` (kilobyte, 1000), `MB` (megabyte, 1000²), `GB` (gigabyte, 1000³), `TB`, `PB`, `EB`
- IEC (binary) units: `KiB` (kibibyte, 1024), `MiB` (mebibyte, 1024²), `GiB` (gibibyte, 1024³), `TiB`, `PiB`, `EiB`

**Examples:**

```yaml
rotation:
  maxSizeBytes: 10MB      # 10,000,000 bytes
  maxSizeBytes: 1GB       # 1,000,000,000 bytes
  maxSizeBytes: 512KiB    # 524,288 bytes
  maxSizeBytes: 1.5GiB    # 1,610,612,736 bytes
```

## Probe Configuration

Readiness and liveness probes support the following handler types:

```yaml
# HTTP GET Probe
handler:
  type: httpGet
  path: /health
  port: 80

# TCP Socket Probe
handler:
  type: tcpSocket
  host: localhost
  port: 5432
```

Probe timing options (durations, e.g., "5s", "1m"):

- `initialDelay`: Time to wait before first probe
- `period`: Interval between probes
- `timeout`: Probe timeout duration
- `failureThreshold`: Consecutive failures before taking action
- `successThreshold`: Consecutive successes before marking healthy

## Restart Policy Types

```yaml
# Never restart
restartPolicy:
  type: Never

# Restart on failure with backoff
restartPolicy:
  type: OnFailure
  maxRetries: 3
  backoff: 5s

# Always restart
restartPolicy:
  type: Always
  backoff: 1s
```

## Shutdown Signal Types

```yaml
# Signal by name
shutdownSignal:
  type: name
  value: SIGTERM

# Signal by number
shutdownSignal:
  type: number
  value: 9

# Default signal (SIGTERM)
shutdownSignal:
  type: sigterm
```

## Log Configuration

Processes can optionally configure logging for their standard output and error streams via the `log` field. The configuration supports three destinations:

- **`null`**: Discard all output (equivalent to `/dev/null`).
- **`inherit`**: Use the supervisor's stdout/stderr (default if no log configuration is provided).
- **`file`**: Write output to a file, with optional rotation.

Log configuration is set per stream (`stdout` and `stderr`) and includes rotation options:

- `maxSizeBytes`: Maximum file size before rotating. Accepts integers (bytes) or human-readable strings (`"10MB"`, `"1GB"`).
- `rotationInterval`: Time-based rotation interval as a duration (e.g., `"24h"`, `"1h"`).
- `maxFiles`: Maximum number of rotated files to retain (older files are deleted).
- `maxAgeDays`: Maximum age in days for rotated files before automatic deletion.
- `mode`: File creation permissions as an octal string (e.g., `"644"`, `"600"`).
- `compression`: Compression algorithm for rotated logs (`none`, `gzip`, or `lz4`).

Example log configuration:

```yaml
log:
  stdout:
    destination:
      type: file
      path: /var/log/myapp/stdout.log
    rotation:
      maxSizeBytes: 10MB
      rotationInterval: 24h
      maxFiles: 7
      compression: gzip
  stderr:
    destination:
      type: inherit
```

If no `log` section is specified, both stdout and stderr default to `inherit`.

## Example Configuration

```yaml
version: "1.0"

processes:
  nginx:
    program: /usr/sbin/nginx
    arguments:
      - -g
      - daemon off;
    terminationGracePeriod: 60s
    readinessProbe:
      handler:
        type: httpGet
        path: /health
        port: 80
      initialDelay: 5s
      period: 10s
      timeout: 1s
      failureThreshold: 3
      successThreshold: 1
    livenessProbe:
      handler:
        type: tcpSocket
        host: localhost
        port: 80
      period: 30s
      timeout: 5s
      failureThreshold: 3
    restartPolicy:
      type: Never
    shutdownSignal:
      type: sigterm

  myapp:
    program: /usr/bin/myapp
    arguments:
      - --config
      - /etc/myapp/config.yaml
    environmentVariables:
      LOG_LEVEL: info
      DATABASE_URL: postgres://localhost:5432/mydb
    workingDirectory: /opt/myapp
    terminationGracePeriod: 30s
    shutdownSignal:
      type: name
      value: SIGTERM
    restartPolicy:
      type: OnFailure
      maxRetries: 3
      backoff: 5s

  redis:
    program: /usr/bin/redis-server
    arguments:
      - --port
      - 6379
    terminationGracePeriod: 10s
    shutdownSignal:
      type: number
      value: 9
    restartPolicy:
      type: Always
      backoff: 1s
```
