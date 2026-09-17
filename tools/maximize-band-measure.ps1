# One-off diagnostic (walkthrough: middle panel not filling vertically when maximized).
# Measures Win32 rects of dev app (target\debug\app.exe) main window: outer GetWindowRect vs
# client GetClientRect, normal vs maximized. NC vertical total = outer.H - client.H:
# normal should ~= caption+2*frame; if maximized NC total stays that big (caption still
# subtracted) the client area is short at the bottom = the reported band, deficit = band px.
# Measure only, no changes; restores the window at the end.
$ErrorActionPreference = "Stop"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32 {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowLongPtrW(IntPtr h, int i);
}
public struct RECT { public int Left, Top, Right, Bottom;
  public int W { get { return Right - Left; } } public int H { get { return Bottom - Top; } } }
"@

$proc = Get-Process -Name app -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$hwnd = $proc.MainWindowHandle
Write-Host "app.exe PID=$($proc.Id) HWND=$hwnd"

function Measure($tag) {
  $wr = New-Object RECT; $cr = New-Object RECT
  [U32]::GetWindowRect($hwnd, [ref]$wr) | Out-Null
  [U32]::GetClientRect($hwnd, [ref]$cr) | Out-Null
  $nc = $wr.H - $cr.H
  $style = [U32]::GetWindowLongPtrW($hwnd, -16)  # GWL_STYLE
  $caption = ($style -band 0x00C00000) -eq 0x00C00000
  Write-Host ("{0}: outer {1}x{2} @({3},{4}) | client {5}x{6} | NC-v={7}px | WS_CAPTION={8} | Zoomed={9}" -f `
    $tag, $wr.W, $wr.H, $wr.Left, $wr.Top, $cr.W, $cr.H, $nc, $caption, [U32]::IsZoomed($hwnd))
}

Measure "normal"
[U32]::ShowWindowAsync($hwnd, 3) | Out-Null  # SW_MAXIMIZE
Start-Sleep -Milliseconds 1500
Measure "maximized"
[U32]::ShowWindowAsync($hwnd, 9) | Out-Null  # SW_RESTORE
Start-Sleep -Milliseconds 800
Measure "restored"
