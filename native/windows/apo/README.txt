Voxveil Windows System Audio Component (development build)
=========================================================

Voxveil embeds its Windows system-audio payload inside voxveil.exe. On Windows 11, the installer
uses the componentized Audio Processing Object (APO) model: an AudioProcessingObject package provides
VoxveilApo.dll, and a target-specific Extension package associates that APO with the current default
render device. Voxveil Output / SYSVAD is not required in APO mode.

Recommended installation
------------------------
1. Make the speakers/headphones you want Voxveil to process the Windows default output device.
2. Start voxveil.exe.
3. Use "Install system audio component".
4. Accept the Windows UAC prompt.
5. Let Windows install the APO and target-specific Extension packages.
6. Restart Windows if Voxveil asks you to or if the audio device cannot be restarted automatically.
7. Play audio, reopen/refocus Voxveil, then enable Processing.

The app extracts the embedded package only while installing. The APO DLL and diagnostic helpers are
also retained under Program Files\Voxveil\system-audio for diagnostics/uninstall support.

What the development installer does
-----------------------------------
- Finds the current default eRender/eMultimedia endpoint and maps it to one physical PnP audio
  function device and its topology interface references.
- Removes Voxveil's older broad runtime FX registration if it is present.
- Generates a target-specific Extension INF for that one render device only.
- Creates local development catalogs for the APO and Extension packages and signs them with a
  temporary machine-local Voxveil development certificate.
- Places the public certificate in the machine Root and TrustedPublisher stores while the development
  installation exists.
- Installs both packages through Windows PnP/Driver Store APIs (pnputil).
- Restarts the target audio device and Windows audio services when possible.

The installer does NOT enable Windows TESTSIGNING, run bcdedit, disable Secure Boot, take ownership of
protected MMDevices keys, or directly write MMDevices\Audio\Render FxProperties.

Development protected-audio note
--------------------------------
VoxveilApo.dll is not yet a production/WHCP-signed APO. For this development build, install.ps1 saves
the existing DisableProtectedAudioDG value and sets DisableProtectedAudioDG=1 so the development APO
can load outside protected AudioDG. uninstall.ps1 restores the previous value exactly. This is separate
from Windows Test Mode and does not change BCD.

Runtime verification
--------------------
Voxveil does not consider the APO ready merely because files were installed. VoxveilApo.dll writes a
small heartbeat under ProgramData\Voxveil only after it has been loaded and APOProcess has actually
received audio. The app reports the APO as ready only when that heartbeat is recent and the processed
buffer count is non-zero.

For manual diagnostics while audio is playing:

  tasklist /m VoxveilApo.dll

A functioning installation should show audiodg.exe loading VoxveilApo.dll. The control file is:

  ProgramData\Voxveil\apo-control.bin

and runtime heartbeat is:

  ProgramData\Voxveil\apo-runtime.bin

Uninstall
---------
Run the installed/embedded uninstall.ps1 as Administrator. It removes Voxveil's Driver Store packages,
removes the machine-local development trust certificate, cleans any legacy runtime registration,
restores the previous DisableProtectedAudioDG value, removes Voxveil control/runtime state, and
restarts the affected audio device/services where possible.

Production packaging
--------------------
This is still a development deployment path. A public production release should ship Microsoft/Windows
properly signed APO and Extension packages instead of creating a local development publisher certificate
on the target machine.

Notes
-----
- Windows 11 build 22000 or newer is required by this package iteration.
- Administrator permission is required for installation/removal.
- The component is installed for the output device that is the Windows default when installation runs.
- If the default output device changes later, reinstall Voxveil system audio for the new device.
- A reboot may be needed if Windows cannot reconstruct/restart the affected endpoint immediately.
- Applications or audio paths that intentionally bypass Windows system effects may not be processed.
