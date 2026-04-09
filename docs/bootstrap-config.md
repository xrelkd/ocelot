# Bootstrap Configuration

The `bootstrap` command uses a YAML configuration file for initramfs-style system initialization. It is designed to mount filesystems, load kernel modules, perform switch_root, and hand off to a supervisor or shell.

> **Note**: The following features are NOT yet fully supported and should not be configured:
>
> - `network` field
> - `security` field (SELinux, AppArmor)
> - `clock` field

Generate the default configuration template with:

```bash
# For shell mode (hands off to interactive shell)
ocelot bootstrap config-template --mode shell

# For supervise mode (hands off to process supervisor)
ocelot bootstrap config-template --mode supervise

# For exec mode (replaces with specified program)
ocelot bootstrap config-template --mode exec
```

## Configuration Structure

The bootstrap configuration has three phases:

### Pre-switch Phase

Executed before `switch_root`. This phase runs in the initramfs environment.

| Field                   | Type   | Description                                  |
| ----------------------- | ------ | -------------------------------------------- |
| `console`               | string | Console device (e.g., "console", "ttyS0")    |
| `logLevel`              | string | Log level (trace, debug, info, warn, error)  |
| `preSwitch.modules`     | object | Kernel modules to load (see Modules section) |
| `preSwitch.mounts`      | list   | Root filesystem mount specification          |
| `preSwitch.hooks`       | list   | Custom commands to execute                   |
| `preSwitch.symlinks`    | list   | Symbolic links to create                     |
| `preSwitch.sysctl`      | map    | Sysctl settings                              |
| `preSwitch.tmpfiles`    | list   | Temporary files to create                    |
| `preSwitch.environment` | map    | Environment variables                        |

> **Note**: Virtual filesystems (sysfs, proc, devtmpfs, tmpfs, devpts) are automatically mounted by bootstrap and should NOT be configured in the `mounts` field.

### Switch-root Phase

The pivot root operation that switches to the real root filesystem.

| Field                       | Type   | Description                         |
| --------------------------- | ------ | ----------------------------------- |
| `switchRoot.rootFileSystem` | object | Root filesystem mount specification |

### Post-switch Phase

Executed after `switch_root` in the real root filesystem.

| Field                | Type   | Description              |
| -------------------- | ------ | ------------------------ |
| `postSwitch.*`       | object | Same fields as preSwitch |
| `postSwitch.handoff` | object | Handoff configuration    |

## Kernel Modules

The modules configuration allows loading kernel modules:

```yaml
preSwitch:
  modules:
    mode: list
    directory: /lib/modules
    names:
      - kernel/fs/9p/9p.ko.xz
      - kernel/net/9p/9pnet.ko.xz
      - kernel/net/9p/9pnet_fd.ko.xz
      - kernel/net/9p/9pnet_virtio.ko.xz
      - kernel/fs/netfs/netfs.ko.xz
      - kernel/drivers/virtio/virtio.ko.xz
      - kernel/drivers/virtio/virtio_pci.ko.xz
      - kernel/drivers/virtio/virtio_pci_legacy_dev.ko.xz
      - kernel/drivers/virtio/virtio_pci_modern_dev.ko.xz
      - kernel/drivers/virtio/virtio_ring.ko.xz
      - kernel/net/core/failover.ko.xz
      - kernel/drivers/net/net_failover.ko.xz
      - kernel/drivers/net/virtio_net.ko.xz
      - kernel/net/packet/af_packet.ko.xz
    depFilePath: modules.dep
```

Fields:

- `mode`: Currently only `list` mode is supported
- `directory`: Directory containing kernel modules (typically `/lib/modules`)
- `names`: List of module paths relative to `directory`
- `depFilePath`: Optional path to modules.dep file for dependency resolution

## Root Filesystem Mount

The root filesystem can be mounted from various sources:

```yaml
# Virtiofs (common for QEMU)
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /
    overlay: false

# Block device
switchRoot:
  rootFileSystem:
    type: device
    device: /dev/vda1
    target: /
    overlay: false

# 9p filesystem
switchRoot:
  rootFileSystem:
    type: 9p
    tag: rootfs
    target: /
    overlay: false

# NFS
switchRoot:
  rootFileSystem:
    type: nfs
    server: 192.168.1.1
    export: /exports/rootfs
    target: /
    overlay: false

# Overlay filesystem
switchRoot:
  rootFileSystem:
    type: overlay
    lower: /lower
    upper: /upper
    work: /work
    target: /
```

Mount options:

```yaml
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /
    overlay: false
    readOnly: true # MS_RDONLY
    noExec: true # MS_NOEXEC
    noSuid: true # MS_NOSUID
    noDev: true # MS_NODEV
    sync: true # MS_SYNCHRONOUS
    dirSync: true # MS_DIRSYNC
    mandatoryLocks: true # MS_MANDLOCK
    posixAcl: true # MS_POSIXACL
    atime: noAtime # AtimeMode: noAtime, relAtime, strictAtime, lazyTime
```

## Handoff Modes

After bootstrap completes, the system can hand off control in three ways:

### Supervise Mode

Hands off to the process supervisor to manage application processes:

```yaml
postSwitch:
  handoff:
    mode: supervise
    processes:
      init:
        program: /sbin/init
      nginx:
        program: /usr/bin/nginx
        arguments:
          - -g
          - daemon off;
```

### Shell Mode

Spawns an interactive shell:

```yaml
postSwitch:
  handoff:
    mode: shell
    program: /bin/sh
```

### Exec Mode

Replaces the bootstrap process with a specific program:

```yaml
postSwitch:
  handoff:
    mode: exec
    program: /usr/bin/nginx
    arguments:
      - -g
      - daemon off;
```

## Boot Script

A boot script can be executed before handoff:

```yaml
postSwitch:
  handoff:
    mode: supervise
    bootScript:
      path: /path/to/a/script.sh
      # Hook failure policy: Warn, Abort, or Retry
      onFailure: Warn
```

## Example Configuration

### Shell Mode Example

```yaml
# Ocelot Bootstrap Configuration (Shell Mode)

console: console
logLevel: info

preSwitch:
  modules: null
  mounts: []
  environment: {}
  symlinks: []
  hooks: []
  sysctl: {}
  tmpfiles: null

switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /
    overlay: false
    readOnly: true
    noExec: true

postSwitch:
  handoff:
    mode: shell
    program: /bin/sh
```

### Supervise Mode Example

```yaml
# Ocelot Bootstrap Configuration (Supervise Mode)

console: console
logLevel: info

preSwitch:
  modules: null
  mounts: []
  environment: {}
  symlinks: []
  hooks: []
  sysctl: {}
  tmpfiles: null

switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /
    overlay: false

postSwitch:
  environment:
    POD_NAMESPACE: default
  sysctl:
    net.core.somaxconn: 1024

  handoff:
    mode: supervise
    processes:
      init:
        program: /sbin/init
    http-server:
      program: /usr/bin/nginx
      arguments:
        - -g
        - daemon off;
```

## Validating Configuration

Validate your bootstrap configuration file:

```bash
ocelot bootstrap validate config.yaml

# For machine-readable output
ocelot bootstrap validate --output json config.yaml
```
