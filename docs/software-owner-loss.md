# Software operations after a daemon restart

The GTK consumer pins the owner-bound `lyra-vega-dbus` implementation at
`167fa31b1dd46414aa5160932a8c45f3123a09bc`, with matching Cargo.lock. It keeps
subscribing before starting each operation and uses `next_transaction(id)`
for individual packages, queues, repository addition and general software
operations. The shared client binds requests and signals to one unique bus
owner, rejects owner replacement and bounds observation to two hours.

An interrupted or expired observation means the result is unconfirmed. The
UI ends its progress presentation, exposes the diagnostic and never marks a
replacement daemon's result as success. Detail errors close the transaction
state with one alert. A queue stops on method/stream errors and reports the
remaining packages as not started; a confirmed unsuccessful completion can
still continue with the next package. There is no automatic retry or cancel.

Passive dashboard and notification listeners create a fresh connection after
loss, with a delay. The notifier keeps a single periodic read-only fallback;
transaction monitoring never shares the reconnect loop. New diagnostics have
pt-BR, en-US and es-ES translations, including the library's typed errors.

## Validation

- Shared-client private D-Bus tests reproduce daemon disappearance, replacement
  reusing an ID, replacement between subscription and mutation, bus death,
  early completion, sequential transactions and deadline persistence. The old
  main accepted replacement `Finished(id=1, success=true)` as the old result.
- GTK compiles against the pinned Git revision with the lockfile. Rust tests,
  fmt and Clippy pass; translation subprocesses check the new typed messages in
  all three locales. Python checks cover the complete translation catalogs.
- The library's daemon-contract CI runs the real vegad on the same openSUSE
  container used by vegad's own CI; Ubuntu was unsupported by distro detection.

No test starts a package transaction on the host. The private daemon uses fake
Software methods; these tests do not qualify real RPM operations or a complete
GNOME login. Packaging versions remain unchanged pending the next RPM release;
this change does not publish an OBS package or install the candidate locally.
