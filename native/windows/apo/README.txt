Voxveil Windows System Audio Component (development build)
=========================================================

This package installs VoxveilApo.dll as a Windows endpoint Audio Processing Object (APO).
It processes the physical render endpoints directly; Voxveil Output / SYSVAD is not required in APO mode.

Recommended installation
------------------------
1. Keep this system-audio folder beside voxveil.exe.
2. Start Voxveil.
3. Use "Install system audio component" when Voxveil reports that the component is missing.
4. Accept the Windows UAC prompt.
5. Restart Voxveil if requested, then enable Processing.

Manual installation
-------------------
Open PowerShell or Terminal and run:

  powershell.exe -ExecutionPolicy Bypass -File .\install.ps1

The script will request Administrator elevation when needed.

Development security note
-------------------------
This GitHub Actions build is not production code-signed. To allow development APO loading,
install.ps1 sets the Windows test/development value DisableProtectedAudioDG=1 and records its
previous state. uninstall.ps1 restores the previous value exactly.

The installer also backs up existing endpoint-effect registry values before adding Voxveil as a
composite endpoint effect. It temporarily enables Windows system effects/audio enhancements on
those endpoints so the APO is not bypassed. uninstall.ps1 restores the previous endpoint effect
registrations and each endpoint's prior system-effects setting.

Uninstall
---------
Run:

  powershell.exe -ExecutionPolicy Bypass -File .\uninstall.ps1

The script requests elevation, removes Voxveil APO registration, restores prior endpoint effects,
restores endpoint system-effects preferences, restores DisableProtectedAudioDG, and restarts
Windows audio services where possible.

Notes
-----
- Administrator permission is required for installation/removal.
- A reboot may be needed if Windows audio services cannot restart automatically.
- Applications that deliberately use audio paths which bypass system effects may not be processed.
- Production distribution should replace this development registration with a properly signed,
  componentized Windows audio deployment package.
