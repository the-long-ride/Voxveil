Voxveil Windows System Audio Component (development build)
=========================================================

Voxveil embeds the development runtime payload inside voxveil.exe. The app installs VoxveilApo.dll
as an endpoint Audio Processing Object (APO), registers the development COM/AudioEngine metadata,
and associates the APO with KSCATEGORY_AUDIO topology interfaces at runtime. Voxveil Output / SYSVAD
is not required in APO mode.

Recommended installation
------------------------
1. Start voxveil.exe.
2. Use "Install system audio component" when Voxveil reports that the component is missing.
3. Accept the Windows UAC prompt.
4. Restart Voxveil (or Windows if requested), then enable Processing.

The app extracts the embedded payload only while installing. The persistent APO DLL and helper are
copied to Program Files\Voxveil\system-audio automatically.

Development security note
-------------------------
This is deliberately a development installation path, not the production signed componentized APO
package. It does not install a driver package, modify Windows boot configuration, enable Test Mode,
change Secure Boot, take ownership of protected endpoint registry keys, or write MMDevices
FxProperties directly.

The helper uses SetupAPI to open the registry storage owned by KSCATEGORY_AUDIO device interfaces.
It appends Voxveil to the composite endpoint-effect list and default processing mode while preserving
existing values. Marker values record only what Voxveil added so uninstall can remove Voxveil without
deliberately replacing an OEM endpoint effect.

The unsigned development APO cannot run in protected audiodg. install.ps1 therefore sets the Windows
development value DisableProtectedAudioDG=1 after saving its previous state. uninstall.ps1 restores
that previous state exactly. This is separate from Windows Test Mode and does not change BCD.

Manual installation (development only)
--------------------------------------
If you extracted the embedded runtime payload for debugging, run:

  powershell.exe -ExecutionPolicy Bypass -File .\install.ps1

The script requests Administrator elevation when needed.

Uninstall
---------
Run the embedded/development uninstall.ps1 as Administrator. It removes Voxveil from the runtime
audio-interface FX properties, removes only Voxveil's development COM/APO registration, restores
DisableProtectedAudioDG, and restarts the Windows audio stack where possible.

Production packaging
--------------------
VoxveilApo.inf and extension.ps1 remain in source as the production direction. A production release
should use the Windows componentized APO + Extension INF model with proper catalog/signing rather
than the development runtime/global registration path.

Notes
-----
- Windows 11 build 22000 or newer is required by this package iteration.
- Administrator permission is required for installation/removal.
- A reboot may be needed if Windows cannot restart the affected audio services immediately.
- Applications or endpoints that deliberately bypass system effects may not be processed.
