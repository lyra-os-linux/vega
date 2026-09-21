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
state. Removal preserves disks and refuses advanced metadata/NVRAM cases that
require special handling. No recursive filesystem deletion exists.

CPU/RAM editing is available for stopped persistent machines without saved state.
Advanced topology/NUMA/pinning/hotplug definitions are rejected, preserving their
configuration. Other XML, especially disks, stays byte-for-byte unchanged.
Launching `lyra-vms` without a UUID opens Vega's VM page, including when Vega
is already running, through the new `--virtual-machines` application option.

Initial creation is personal-only, from ISO, BIOS/q35/x86_64/KVM. It copies media
into an owned volume and creates a qcow2 disk under the XDG data directory.
An existing pool pointing elsewhere is rejected. Rollback only touches newly
created volumes; incomplete cleanup is reported. Created machines stay off.
VNC uses a Unix socket with libvirt attachment, not a public TCP listener.

## Still required before publishing

- Mid-copy cancellation/failure rollback and console keyboard/fullscreen tests.
- Disk import, UEFI selection and direct per-machine shortcuts.
- Translate backend diagnostics; accessibility/narrow-window review.
- Source vendoring/RPM/OBS gates and recipe integration.
- Audit before ISO; exact-checksum candidate qualification afterward.

The libvirt test driver covers lifecycle and refuses invalid state transitions.
It does not qualify a QEMU console or installation. Keep virt-manager installed
until the replacement passes those gates. Do not mark Alpha 8 qualified yet.

The disposable QEMU probe also verifies creation, early cancellation, duplicate
names, KVM start, pause/resume, removal and original media/disk preservation.
Its media is a nonbootable fixture: it does not prove a guest OS installation.
