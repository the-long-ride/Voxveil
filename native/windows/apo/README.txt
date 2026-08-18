Voxveil Windows System Audio Component (development build)
=========================================================

Voxveil embeds this package inside voxveil.exe. The package installs VoxveilApo.dll as a Windows
endpoint Audio Processing Object (APO) using the Windows componentized APO + Extension INF model.
Voxveil Output / SYSVAD is not required in APO mode.

Recommended installation
------------------------
1. Start voxveil.exe.
2. Use "Install system audio component" when Voxveil reports that the component is missing.
3. Accept the Windows UAC prompt.
4. Windows may also ask you to approve the development driver publisher/package.
5. Restart Voxveil (or Windows if requested), then enable Processing.

The app extracts the embedded files only while installing. You do not need to manage a separate
system-audio folder.

Development security note
-------------------------
This development package is not a production Microsoft-signed driver package. Windows therefore
controls whether it will stage the APO and Extension INF packages and may require explicit
administrator approval. Voxveil does not change Windows boot configuration, Secure Boot settings,
or protected endpoint-registry ACLs.

To allow the unsigned development APO DLL to run outside protected audiodg, install.ps1 sets the
Windows development value DisableProtectedAudioDG=1 and records its previous state. uninstall.ps1
restores that previous value exactly.

The Extension INF appends Voxveil to the composite endpoint-effect list through Windows PnP so it
does not deliberately replace an existing OEM endpoint effect. Windows Endpoint Builder owns the
resulting effects property store.

Manual installation (development only)
--------------------------------------
If you extracted the embedded package for debugging, run:

  powershell.exe -ExecutionPolicy Bypass -File .\install.ps1

The script requests Administrator elevation when needed.

Uninstall
---------
Run the embedded/development uninstall.ps1 as Administrator. It removes the Voxveil PnP packages,
restores DisableProtectedAudioDG, cleans the old pre-INF Voxveil global registration if present,
and restarts the Windows audio stack where possible.

Notes
-----
- Windows 11 build 22000 or newer is required by this package iteration.
- Administrator permission is required for installation/removal.
- A reboot may be needed if Windows cannot restart the affected audio devices/services immediately.
- Applications that deliberately use audio paths which bypass system effects may not be processed.
- Production distribution requires stable release packages signed through the Windows driver
  distribution/signing process.
