# PKG-03 03.09 Research — Desktop registration lifecycle

Reviewed: 2026-08-27
Canonical base: `4f5e8ab30f030e758c52c4ca4ac08f73f896247a`
Linear: `ABD-84`
Change required: **false**

## Current-source findings

- The accepted product config does not define file associations, deep-link protocols, custom WiX templates, or custom NSIS templates.
- Stock Tauri NSIS creates a Start Menu shortcut in the interactive GUI flow. Desktop shortcut creation is user-controlled in the GUI; silent/passive shortcut flags belong to 03.21 and are out of scope here.
- Stock Tauri NSIS associates created shortcuts with the bundle identifier through an AppUserModelID helper and removes its shell destination state during uninstall.
- Stock Tauri WiX defines Start Menu and Desktop shortcut components. The Start Menu shortcut carries `System.AppUserModel.ID` equal to the bundle identifier. WiX shortcut ownership/cleanup is MSI-component driven.
- The accepted bundle identifier remains `dev.vsn.platform`; product name remains `VSN Dev Platform`.
- CLI and Agent paths remain declared but not packaged until 03.10.

## Platform delta

No material platform delta requires product/config mutation for 03.09. The task can certify stock installer behavior with a task-local validator, interactive harness and Windows workflow.

`change_required=false`
