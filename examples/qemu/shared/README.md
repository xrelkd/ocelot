# Shared Directory

This directory is mounted to `/mnt/shared` inside the QEMU VM via 9p.

## Usage

Files placed here are immediately visible in the VM:

```bash
# On host
echo "Hello from host" > examples/qemu/shared/test.txt

# In VM
cat /mnt/shared/test.txt
```

## Use Cases

- Transfer test scripts to the VM
- Share configuration files
- Collect logs or output from VM
- Test file I/O between host and guest
