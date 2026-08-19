Set-StrictMode -Version 2.0

if (-not ('Voxveil.Catalog.Native' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

namespace Voxveil.Catalog
{
    public static class Native
    {
        private const uint CRYPTCAT_VERSION_2 = 0x200;
        private const uint CRYPTCAT_ATTR_AUTHENTICATED = 0x10000000;
        private const uint CRYPTCAT_ATTR_DATAASCII = 0x00010000;
        private const uint CRYPTCAT_ATTR_NAMEASCII = 0x00000001;
        private static readonly IntPtr InvalidHandle = new IntPtr(-1);

        [DllImport("wintrust.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CryptCATOpen(
            string fileName,
            uint openFlags,
            IntPtr provider,
            uint publicVersion,
            uint encodingType);

        [DllImport("wintrust.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern IntPtr CryptCATPutCatAttrInfo(
            IntPtr catalog,
            string referenceTag,
            uint attrTypeAndAction,
            uint dataLength,
            byte[] data);

        [DllImport("wintrust.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptCATPersistStore(IntPtr catalog);

        [DllImport("wintrust.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool CryptCATClose(IntPtr catalog);

        public static void AddOsAttr(string catalogPath, string value)
        {
            IntPtr catalog = CryptCATOpen(catalogPath, 0, IntPtr.Zero, CRYPTCAT_VERSION_2, 0);
            if (catalog == IntPtr.Zero || catalog == InvalidHandle)
            {
                throw new Win32Exception(Marshal.GetLastWin32Error(), "CryptCATOpen failed");
            }

            try
            {
                byte[] data = Encoding.ASCII.GetBytes(value);
                uint flags = CRYPTCAT_ATTR_AUTHENTICATED |
                             CRYPTCAT_ATTR_DATAASCII |
                             CRYPTCAT_ATTR_NAMEASCII;
                IntPtr attribute = CryptCATPutCatAttrInfo(
                    catalog,
                    "OSAttr",
                    flags,
                    checked((uint)data.Length),
                    data);
                if (attribute == IntPtr.Zero)
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "CryptCATPutCatAttrInfo(OSAttr) failed");
                }
                if (!CryptCATPersistStore(catalog))
                {
                    throw new Win32Exception(
                        Marshal.GetLastWin32Error(),
                        "CryptCATPersistStore failed");
                }
            }
            finally
            {
                CryptCATClose(catalog);
            }
        }
    }
}
'@
}

function Add-VoxveilPnpCatalogAttributes {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [string]$OsAttr = '2:10.0'
    )

    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "Voxveil catalog does not exist: $resolved"
    }
    [Voxveil.Catalog.Native]::AddOsAttr($resolved, $OsAttr)
}
