# Fstab Conventions

## Format

```
<file system> <mount point> <type> <options> <dump> <pass>
```

## Common issues

- **Duplicate entries**: same UUID or path listed multiple times — systemd-fstab-generator fails
- **Swapfile entries**: `/home/swapfile none swap defaults 0 0` — btrfs swapfile needs `compress=no` option
- **UUID stale**: filesystem reformat changed UUID — entry won't mount
- **Missing mount**: path doesn't exist — generator creates unit but mount fails

## Fix patterns

- Duplicate swapfile: remove extras with `sudo sed -i 'N,N+1d' /etc/fstab`
- Stale UUID: update with `lsblk -f` or `blkid`
- Missing path: create directory before mounting

## Swapfile on btrfs

btrfs swapfiles need compression disabled:
```
/home/swapfile none swap compress=no 0 0
```
