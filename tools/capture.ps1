# Drive every new binding with REAL scan codes.
#
# miniquad derives the key from the scancode in WM_KEYDOWN's lparam
# (HIWORD & 0x1FF), not the virtual-key code, so synthetic input must carry a
# valid scan code or it translates to nothing.
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class D {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[void][D]::SetProcessDPIAware()

$exe = "D:\Documents\rust-fps\target\release\rust-fps.exe"
$out = "D:\Documents\rust-fps\target\shots"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$proc = $null
try {
    $proc = Start-Process -FilePath $exe -WorkingDirectory "D:\Documents\rust-fps" -PassThru
    $h = [IntPtr]::Zero
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 250
        $proc.Refresh()
        if ($proc.HasExited) { throw "exited early" }
        if ($proc.MainWindowHandle -ne [IntPtr]::Zero) { $h = $proc.MainWindowHandle; break }
    }
    Start-Sleep -Seconds 2

    $r = New-Object D+RECT
    [void][D]::GetWindowRect($h, [ref]$r)
    $cx = [int](($r.Left + $r.Right) / 2)
    $ly = [int]($r.Top + ($r.Bottom - $r.Top) * 0.447)

    # Click "NEW CAMPAIGN".
    [void][D]::SetCursorPos($cx, $ly)
    Start-Sleep -Milliseconds 250
    [D]::mouse_event(0x02, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Milliseconds 80
    [D]::mouse_event(0x04, 0, 0, 0, [IntPtr]::Zero)
    Start-Sleep -Seconds 7

    function Key($vk, $scan, $wait = 900) {
        [D]::keybd_event($vk, $scan, 0, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 70
        [D]::keybd_event($vk, $scan, 0x0002, [IntPtr]::Zero)
        Start-Sleep -Milliseconds $wait
    }

    function Shot($name) {
        $r2 = New-Object D+RECT
        [void][D]::GetWindowRect($h, [ref]$r2)
        $bmp = New-Object System.Drawing.Bitmap ($r2.Right - $r2.Left), ($r2.Bottom - $r2.Top)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($r2.Left, $r2.Top, 0, 0, $bmp.Size)
        $bmp.Save((Join-Path $out $name), [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Host "captured $name"
    }

    Key 0x32 0x32; Shot "40_map.png"; Key 0x32 0x32 700
    Key 0x24 0x24; Shot "41_mission.png"; Key 0x24 0x24 700
    Key 0x05 0x05; Shot "42_knife.png"
    Key 0x53 0x1F; Shot "43_crouch.png"          # S key: crouch lowers the view
    Key 0x53 0x1F 700
    Key 0x43 0x2E; Shot "44_crouch_toggle.png"   # C toggle
    Key 0x43 0x2E 700
    Key 0x30 0x30; Shot "45_firemode.png"        # B cycles fire mode (toast)
    Key 0x16 0x16; Shot "46_inspect.png"         # U inspect animation

    $proc.Refresh()
    if ($proc.HasExited) { throw "exited during input, code $($proc.ExitCode)" }
    Write-Host "OK: every binding exercised"
}
finally {
    if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force; Write-Host "stopped" }
}
