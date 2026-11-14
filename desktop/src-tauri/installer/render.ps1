# Draws the installer's pictures from their SVG sources: sidebar.bmp and header.bmp.
# Needs Microsoft Edge (for the drawing) and ffmpeg (for scaling and the BMP format).
#   powershell -ExecutionPolicy Bypass -File render.ps1
# The pictures are saved at twice the size the installer asks for (164 x 314 and 150 x 57),
# so they stay sharp on displays scaled to 150 % and 200 %.
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge was not found.' }
$work = Join-Path ([System.IO.Path]::GetTempPath()) "snapintel-installer-art"
New-Item -ItemType Directory -Force $work | Out-Null

foreach ($picture in @(@{ name = 'sidebar'; width = 164; height = 314 }, @{ name = 'header'; width = 150; height = 57 })) {
    $png = Join-Path $work "$($picture.name).png"
    $source = ([System.Uri](Join-Path $here "$($picture.name).svg")).AbsoluteUri
    # Drawn at four times the size, then scaled down, for clean edges.
    $arguments = @('--headless=new', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=4', "--window-size=$($picture.width),$($picture.height)", "--user-data-dir=$work\profile", "--screenshot=$png", $source)
    Start-Process $edge -ArgumentList $arguments -Wait -WindowStyle Hidden
    if (-not (Test-Path $png)) { throw "Edge did not draw $($picture.name)." }
    $size = "$($picture.width * 2):$($picture.height * 2)"
    & ffmpeg -hide_banner -loglevel error -y -i $png -vf "crop=$($picture.width * 4):$($picture.height * 4):0:0,scale=${size}:flags=lanczos" -pix_fmt bgr24 (Join-Path $here "$($picture.name).bmp")
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg failed on $($picture.name)." }
    "$($picture.name).bmp  $size"
}
