# EFI Boot Manager Schema

## efibootmgr output format

| Field | Meaning |
|---|---|
| BootCurrent | active boot entry number |
| Timeout | boot menu timeout seconds |
| BootOrder | sequence of boot entry numbers |
| Boot0001* | entry 1, `*` means active/preferred |
| UEFI OS | generic EFI OS entry (your arch-linux-zen.efi) |
| UEFI: PXE IPv4 | network boot via IPv4 DHCP |
| UEFI: PXE IPv6 | network boot via IPv6 static |
| PciRoot(0x0)/Pci(...) | PCI path to network controller |
| MAC(...) | MAC address of controller |
| HD(1,GPT,...) | boot partition path |

## efibootmgr commands

| Command | Effect |
|---|---|
| `efibootmgr` | list all entries |
| `efibootmgr -b <num> -B` | remove entry `<num>` from boot order |
| `efibootmgr -b <num> -b` | add entry `<num>` to boot order |
| `efibootmgr -t <sec>` | set timeout |
| `efibootmgr -o <order>` | set boot order sequence |

## Persistence

- EFI entries stored in firmware NVRAM (efivars)
- Persist across reboots, not on disk
- BIOS reset does not recreate entries — manual efibootmgr needed
- Entries survive filesystem changes

## PXE entries on Asus TUF Z690

- Intel I225-V Ethernet controller
- Both IPv4 and IPv6 PXE entries present by default
- Firmware supports WOL/network boot capability
- Disabling in BIOS removes capability but not entries
