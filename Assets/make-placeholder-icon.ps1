<#
Draws the placeholder app icon (a sun behind a cloud on a blue rounded
square) at every size Windows asks for and packs them into Weatherspell.ico
(PNG at 256 px, 32-bit DIBs below). Dev-time only; the committed .ico is what the build
embeds. Needs nothing beyond Windows PowerShell 5.1 (GDI+ is in-box).

    powershell -NoProfile -File Assets\make-placeholder-icon.ps1

Replace this with a designed icon before v0.1.0 (issue #8).
#>
Add-Type -AssemblyName System.Drawing

$sizes = 16, 20, 24, 32, 48, 64, 256
$out = Join-Path $PSScriptRoot 'Weatherspell.ico'

function Draw-Frame([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.PixelOffsetMode = 'HighQuality'
    $g.Clear([System.Drawing.Color]::Transparent)

    $s = [float]$size

    # Background: rounded square, deep-to-light blue.
    $radius = $s * 0.22
    $rect = New-Object System.Drawing.RectangleF 0, 0, $s, $s
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
    $path.AddArc($rect.Right - $d, $rect.Y, $d, $d, 270, 90)
    $path.AddArc($rect.Right - $d, $rect.Bottom - $d, $d, $d, 0, 90)
    $path.AddArc($rect.X, $rect.Bottom - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    $bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.PointF 0, 0), (New-Object System.Drawing.PointF 0, $s),
        [System.Drawing.Color]::FromArgb(255, 30, 100, 190), [System.Drawing.Color]::FromArgb(255, 70, 160, 230))
    $g.FillPath($bg, $path)

    # Sun: upper right, warm yellow with a soft rim.
    $sunR = $s * 0.20
    $sunCx = $s * 0.64; $sunCy = $s * 0.36
    $rim = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 200, 60))
    $g.FillEllipse($rim, $sunCx - $sunR * 1.12, $sunCy - $sunR * 1.12, $sunR * 2.24, $sunR * 2.24)
    $sun = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 225, 100))
    $g.FillEllipse($sun, $sunCx - $sunR, $sunCy - $sunR, $sunR * 2, $sunR * 2)

    # Cloud: three lobes over a flat base, lower left, in front of the sun.
    $cloud = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 250, 252, 255))
    $baseY = $s * 0.70
    $g.FillEllipse($cloud, $s * 0.14, $s * 0.46, $s * 0.30, $s * 0.30)
    $g.FillEllipse($cloud, $s * 0.30, $s * 0.36, $s * 0.36, $s * 0.36)
    $g.FillEllipse($cloud, $s * 0.50, $s * 0.48, $s * 0.28, $s * 0.28)
    $g.FillRectangle($cloud, $s * 0.20, $s * 0.58, $s * 0.54, $baseY - $s * 0.58)
    $baseD = $baseY - $s * 0.58
    $g.FillEllipse($cloud, $s * 0.20 - $baseD / 2, $s * 0.58, $baseD, $baseD)
    $g.FillEllipse($cloud, $s * 0.74 - $baseD / 2, $s * 0.58, $baseD, $baseD)

    $g.Dispose()
    # 256 px is stored as PNG (the only size Windows expects compressed);
    # everything smaller is a classic 32-bit DIB, because System.Drawing.Icon
    # on .NET Framework, which Form.Icon relies on, mis-decodes PNG frames
    # below 256 px.
    if ($size -ge 256) {
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $bytes = $ms.ToArray()
    } else {
        $bytes = Encode-Dib $bmp
    }
    $bmp.Dispose()
    Write-Output -NoEnumerate $bytes
}

# ICO DIB frame: BITMAPINFOHEADER with doubled height, BGRA rows bottom-up,
# then a 1-bit AND mask (rows padded to 4 bytes) marking fully transparent
# pixels for consumers that ignore alpha.
function Encode-Dib([System.Drawing.Bitmap]$bmp) {
    $w = $bmp.Width; $h = $bmp.Height
    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $data = $bmp.LockBits($rect, 'ReadOnly', 'Format32bppArgb')
    $stride = $data.Stride
    $src = New-Object byte[] ($stride * $h)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $src, 0, $src.Length)
    $bmp.UnlockBits($data)

    $maskStride = [int][math]::Ceiling($w / 32.0) * 4
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter $ms
    $bw.Write([uint32]40)                       # biSize
    $bw.Write([int32]$w)                        # biWidth
    $bw.Write([int32]($h * 2))                  # biHeight (XOR + AND)
    $bw.Write([uint16]1)                        # biPlanes
    $bw.Write([uint16]32)                       # biBitCount
    $bw.Write([uint32]0)                        # biCompression BI_RGB
    $bw.Write([uint32]($w * $h * 4 + $maskStride * $h))
    $bw.Write([int32]0); $bw.Write([int32]0)    # pels per metre
    $bw.Write([uint32]0); $bw.Write([uint32]0)  # colours used / important

    for ($y = $h - 1; $y -ge 0; $y--) {
        $bw.Write($src, $y * $stride, $w * 4)
    }
    for ($y = $h - 1; $y -ge 0; $y--) {
        $row = New-Object byte[] $maskStride
        for ($x = 0; $x -lt $w; $x++) {
            $alpha = $src[$y * $stride + $x * 4 + 3]
            if ($alpha -eq 0) { $row[$x -shr 3] = $row[$x -shr 3] -bor (0x80 -shr ($x -band 7)) }  # [int] would round, not truncate
        }
        $bw.Write($row)
    }
    $bw.Flush()
    Write-Output -NoEnumerate $ms.ToArray()
}

$frames = foreach ($size in $sizes) { ,@($size, (Draw-Frame $size)) }

# ICO container: ICONDIR, one ICONDIRENTRY per frame, then the payloads.
$stream = [System.IO.File]::Create($out)
$w = New-Object System.IO.BinaryWriter $stream
$w.Write([uint16]0)               # reserved
$w.Write([uint16]1)               # type: icon
$w.Write([uint16]$frames.Count)
$offset = 6 + 16 * $frames.Count
foreach ($f in $frames) {
    $size = $f[0]; $png = $f[1]
    $dim = if ($size -ge 256) { 0 } else { $size }   # 0 means 256
    $w.Write([byte]$dim)          # width
    $w.Write([byte]$dim)          # height
    $w.Write([byte]0)             # colour count (0 = no palette)
    $w.Write([byte]0)             # reserved
    $w.Write([uint16]1)           # planes
    $w.Write([uint16]32)          # bits per pixel
    $w.Write([uint32]$png.Length) # bytes in resource
    $w.Write([uint32]$offset)     # offset of the image data
    $offset += $png.Length
}
# Cast matters: an Object[] of bytes would resolve to a one-byte overload.
foreach ($f in $frames) { $w.Write([byte[]]$f[1]) }
$w.Flush(); $stream.Close()

"Wrote $out ($((Get-Item $out).Length) bytes, sizes: $($sizes -join ', '))"
