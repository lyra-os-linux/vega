# Virtual machines — Alpha 8 implementation

Approved product division: Vega manages machines, Lyra VMs opens their display,
libvirt/QEMU execute them. Closing either interface does not shut down a guest.

`vega-virtualization` is a separate workspace crate. The page opens no libvirt
connection until mapped. Requests run on worker threads. Personal VMs use
`qemu:///session`; system VMs are a separate `qemu:///system` connection. Reads
open read-only connections; changes use libvirt's own authorization/Polkit.

This is an intentional exception to the vegad-only client convention: a root
vegad would address the wrong user session. No privileged helper, extra Polkit
allow rule, or root GTK process is introduced. Libvirt remains the authorization
boundary, including for system domains. Errors are surfaced, not bypassed.

The page lists machines, filters names, refreshes when opened and every five
seconds while visible, opens the console, starts/shuts down/pauses/resumes, and
confirms force-stop/removal. Every operation re-resolves the UUID and rechecks
state. Removal preserves disks and UEFI NVRAM, and refuses saved state/snapshot
metadata requiring special handling. No recursive filesystem deletion exists.

CPU/RAM editing is available for stopped persistent machines without saved state.
Advanced topology/NUMA/pinning/hotplug definitions are rejected, preserving their
configuration. Other XML, especially disks, stays byte-for-byte unchanged.
Launching `lyra-vms` without a UUID opens Vega's VM page, including when Vega
is already running, through the new `--virtual-machines` application option.

Creation is personal-only, from ISO or a copied QCOW2/RAW disk, with BIOS or UEFI
on q35/x86_64/KVM. ISO installation creates a qcow2 disk under the XDG data directory.
QCOW2 imports reject backing files, encryption, snapshots and advanced feature
flags; QEMU validates the complete format at start. Shut down the source VM
before copying. Progress and cancellation are available during the transfer.
An existing pool pointing elsewhere is rejected. Rollback only touches newly
created volumes; incomplete cleanup is reported. Created machines stay off.
VNC uses a Unix socket with libvirt attachment, not a public TCP listener.

Per-machine desktop shortcuts open the console by UUID. Start the machine in
Vega first; the console waits if it is off. Existing shortcuts are not replaced.

The disposable probe covers RAW/QCOW2 imports, UEFI start/NVRAM preservation,
mid-copy cancellation and injected source truncation. These are component tests,
not qualification of a complete guest OS installation.

## Still required before publishing

- Accessibility/narrow-window and candidate GNOME/Wayland review.
- Source vendoring/RPM/OBS gates and recipe integration.
- Audit before ISO; exact-checksum candidate qualification afterward.

The libvirt test driver covers lifecycle and refuses invalid state transitions.
It does not qualify a QEMU console or installation. Keep virt-manager installed
until the replacement passes those gates. Do not mark Alpha 8 qualified yet.

The disposable QEMU probe also verifies creation, early cancellation, duplicate
names, KVM start, pause/resume, removal and original media/disk preservation.
Its media is a nonbootable fixture: it does not prove a guest OS installation.

The boot-sector console fixture passed keyboard echo, fullscreen (1280×800),
reconnection after restart, and normal close with the domain still running.
See `evidence/console-20260921.json`. This does not replace candidate tests.
