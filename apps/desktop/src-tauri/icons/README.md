# Desktop icon resources

`icon.ico` is the Windows resource consumed by the Tauri build. `icon.png` supplies the PNG resource required by the Linux desktop context build. Both use the existing neutral foundation placeholder; replace them together during the final Systems Modeler Pro branding transition.

The PNG is the unmodified 64x64 RGBA PNG payload extracted from the tracked ICO (fourth directory entry, byte offset 2745, length 309). No new artwork or external asset is introduced. PNG SHA-256: `90e1def53ebe2f8190da11ee3116c7aefdf4540439a3d263cab295dc12cc4b4f`.

The `desktop-linux-check` CI job compiles the desktop package on Ubuntu; the existing Windows desktop check remains in place. Local hosted qualification can use `cargo check --offline --locked -p systems-modeler-desktop` after dependency preparation. Compile success is not visual or agent-isolation qualification.
