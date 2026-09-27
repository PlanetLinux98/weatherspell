<#
Draws the app icon and writes Weatherspell.ico plus SVG copies of the design.

The design (a cloud and sun above lines of text, falling like rain) lives
here as simple shapes, three times over: the full design on a 256 grid for
32 px and up, and versions drawn for 24 and 16 px, which keep two lines of
text on whole pixels so they stay sharp. The .ico frames and the SVGs are
both made from these shapes, so they cannot drift apart. Dev-time only; the
committed .ico is what the build embeds. Needs nothing beyond Windows
PowerShell 5.1 (GDI+ is in-box).

    powershell -NoProfile -File Assets\make-icon.ps1 [-PreviewDir <folder>]

-PreviewDir also saves each frame as a PNG, to look at before committing.
#>
param([string]$PreviewDir)

Add-Type -AssemblyName System.Drawing

$skyTop = '#5AB0EE'; $skyBottom = '#1E6CC4'; $sun = '#FFC83D'; $white = '#FFFFFF'

$full = @{ Grid = 256; Shapes = @(
    @{ Kind = 'plate'; X = 8; Y = 8; W = 240; H = 240; R = 56 },
    @{ Kind = 'circle'; Cx = 81; Cy = 77; R = 40; Fill = $sun },
    @{ Kind = 'circle'; Cx = 106; Cy = 110; R = 38; Fill = $white },
    @{ Kind = 'circle'; Cx = 152; Cy = 94; R = 48; Fill = $white },
    @{ Kind = 'circle'; Cx = 196; Cy = 116; R = 32; Fill = $white },
    @{ Kind = 'rect'; X = 66; Y = 108; W = 162; H = 40; R = 20; Fill = $white },
    @{ Kind = 'line'; X1 = 74; Y1 = 176; X2 = 200; Y2 = 176; Width = 14; Round = $true; Stroke = $white },
    @{ Kind = 'line'; X1 = 74; Y1 = 200; X2 = 178; Y2 = 200; Width = 14; Round = $true; Stroke = $white },
    @{ Kind = 'line'; X1 = 74; Y1 = 224; X2 = 136; Y2 = 224; Width = 14; Round = $true; Stroke = $white }
) }

$small24 = @{ Grid = 24; Shapes = @(
    @{ Kind = 'plate'; X = 0.5; Y = 0.5; W = 23; H = 23; R = 5.5 },
    @{ Kind = 'circle'; Cx = 7.2; Cy = 7; R = 4.3; Fill = $sun },
    @{ Kind = 'circle'; Cx = 9.9; Cy = 10.3; R = 3.6; Fill = $white },
    @{ Kind = 'circle'; Cx = 14.3; Cy = 8.8; R = 4.5; Fill = $white },
    @{ Kind = 'circle'; Cx = 18.4; Cy = 10.9; R = 3; Fill = $white },
    @{ Kind = 'rect'; X = 6.2; Y = 10; W = 15.2; H = 4; R = 2; Fill = $white },
    @{ Kind = 'line'; X1 = 7; Y1 = 17; X2 = 18; Y2 = 17; Width = 2; Round = $true; Stroke = $white },
    @{ Kind = 'line'; X1 = 7; Y1 = 21; X2 = 13.5; Y2 = 21; Width = 2; Round = $true; Stroke = $white }
) }

$small16 = @{ Grid = 16; Shapes = @(
    @{ Kind = 'plate'; X = 0.5; Y = 0.5; W = 15; H = 15; R = 3.5 },
    @{ Kind = 'circle'; Cx = 5; Cy = 4.8; R = 2.9; Fill = $sun },
    @{ Kind = 'circle'; Cx = 6.6; Cy = 6.9; R = 2.4; Fill = $white },
    @{ Kind = 'circle'; Cx = 9.5; Cy = 5.9; R = 3; Fill = $white },
    @{ Kind = 'circle'; Cx = 12.3; Cy = 7.3; R = 2; Fill = $white },
    @{ Kind = 'rect'; X = 4; Y = 7; W = 10.3; H = 3; R = 1.5; Fill = $white },
    @{ Kind = 'line'; X1 = 4; Y1 = 12.5; X2 = 12; Y2 = 12.5; Width = 1; Round = $false; Stroke = $white },
    @{ Kind = 'line'; X1 = 4; Y1 = 14.5; X2 = 9; Y2 = 14.5; Width = 1; Round = $false; Stroke = $white }
) }

# 20 px takes the 24 px drawing: its lines survive the small scale better
# than the full design's three.
$frames = @(
    @{ Size = 16; Design = $small16 }, @{ Size = 20; Design = $small24 }, @{ Size = 24; Design = $small24 },
    @{ Size = 32; Design = $full }, @{ Size = 36; Design = $full }, @{ Size = 40; Design = $full },
    @{ Size = 48; Design = $full }, @{ Size = 64; Design = $full }, @{ Size = 256; Design = $full }
)

$inv = [System.Globalization.CultureInfo]::InvariantCulture
function N($v) { ([double]$v).ToString($inv) }
function Colour($hex) { [System.Drawing.ColorTranslator]::FromHtml($hex) }

function Rounded-Path($x, $y, $w, $h, $r) {
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = [float]($r * 2)
    $path.AddArc([float]$x, [float]$y, $d, $d, 180, 90)
    $path.AddArc([float]($x + $w - $d), [float]$y, $d, $d, 270, 90)
    $path.AddArc([float]($x + $w - $d), [float]($y + $h - $d), $d, $d, 0, 90)
    $path.AddArc([float]$x, [float]($y + $h - $d), $d, $d, 90, 90)
    $path.CloseFigure()
    $path
}

function Draw-Design($design, [int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    # Pixel centres at .5, as in SVG, so whole-pixel lines land on whole pixels.
    $g.PixelOffsetMode = 'Half'
    $g.Clear([System.Drawing.Color]::Transparent)
    $scale = [float]($size / $design.Grid)
    $g.ScaleTransform($scale, $scale)
    foreach ($s in $design.Shapes) {
        switch ($s.Kind) {
            'plate' {
                # A touch past the plate's edges, so antialiased edge pixels
                # never wrap round to the other end of the gradient.
                $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
                    (New-Object System.Drawing.PointF 0, ([float]($s.Y - 1))),
                    (New-Object System.Drawing.PointF 0, ([float]($s.Y + $s.H + 1))),
                    (Colour $skyTop), (Colour $skyBottom))
                $g.FillPath($brush, (Rounded-Path $s.X $s.Y $s.W $s.H $s.R))
            }
            'circle' {
                $brush = New-Object System.Drawing.SolidBrush (Colour $s.Fill)
                $g.FillEllipse($brush, [float]($s.Cx - $s.R), [float]($s.Cy - $s.R), [float]($s.R * 2), [float]($s.R * 2))
            }
            'rect' {
                $brush = New-Object System.Drawing.SolidBrush (Colour $s.Fill)
                $g.FillPath($brush, (Rounded-Path $s.X $s.Y $s.W $s.H $s.R))
            }
            'line' {
                $pen = New-Object System.Drawing.Pen ((Colour $s.Stroke), [float]$s.Width)
                if ($s.Round) { $pen.StartCap = 'Round'; $pen.EndCap = 'Round' }
                $g.DrawLine($pen, [float]$s.X1, [float]$s.Y1, [float]$s.X2, [float]$s.Y2)
            }
        }
    }
    $g.Dispose()
    $bmp
}

function Write-Svg($design, [string]$path) {
    $sb = New-Object System.Text.StringBuilder
    $gr = N $design.Grid
    [void]$sb.AppendLine("<svg xmlns=`"http://www.w3.org/2000/svg`" viewBox=`"0 0 $gr $gr`" width=`"$gr`" height=`"$gr`">")
    [void]$sb.AppendLine("  <defs><linearGradient id=`"sky`" x1=`"0`" y1=`"0`" x2=`"0`" y2=`"1`"><stop offset=`"0`" stop-color=`"$skyTop`"/><stop offset=`"1`" stop-color=`"$skyBottom`"/></linearGradient></defs>")
    foreach ($s in $design.Shapes) {
        $line = switch ($s.Kind) {
            'plate' { "<rect x=`"$(N $s.X)`" y=`"$(N $s.Y)`" width=`"$(N $s.W)`" height=`"$(N $s.H)`" rx=`"$(N $s.R)`" fill=`"url(#sky)`"/>" }
            'circle' { "<circle cx=`"$(N $s.Cx)`" cy=`"$(N $s.Cy)`" r=`"$(N $s.R)`" fill=`"$($s.Fill)`"/>" }
            'rect' { "<rect x=`"$(N $s.X)`" y=`"$(N $s.Y)`" width=`"$(N $s.W)`" height=`"$(N $s.H)`" rx=`"$(N $s.R)`" fill=`"$($s.Fill)`"/>" }
            'line' {
                $cap = if ($s.Round) { ' stroke-linecap="round"' } else { '' }
                "<line x1=`"$(N $s.X1)`" y1=`"$(N $s.Y1)`" x2=`"$(N $s.X2)`" y2=`"$(N $s.Y2)`" stroke=`"$($s.Stroke)`" stroke-width=`"$(N $s.Width)`"$cap/>"
            }
        }
        [void]$sb.AppendLine("  $line")
    }
    [void]$sb.AppendLine('</svg>')
    [System.IO.File]::WriteAllText($path, $sb.ToString(), (New-Object System.Text.UTF8Encoding $false))
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

if ($PreviewDir) { New-Item -ItemType Directory -Force $PreviewDir | Out-Null }

$payloads = foreach ($f in $frames) {
    $bmp = Draw-Design $f.Design $f.Size
    if ($PreviewDir) { $bmp.Save((Join-Path $PreviewDir "icon-$($f.Size).png"), [System.Drawing.Imaging.ImageFormat]::Png) }
    # 256 px is stored as PNG (the only size Windows expects compressed);
    # everything smaller is a classic 32-bit DIB, because System.Drawing.Icon
    # on .NET Framework, which Form.Icon relies on, mis-decodes PNG frames
    # below 256 px.
    if ($f.Size -ge 256) {
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $bytes = $ms.ToArray()
    } else {
        $bytes = Encode-Dib $bmp
    }
    $bmp.Dispose()
    ,@($f.Size, $bytes)
}

# ICO container: ICONDIR, one ICONDIRENTRY per frame, then the payloads.
$out = Join-Path $PSScriptRoot 'Weatherspell.ico'
$stream = [System.IO.File]::Create($out)
$w = New-Object System.IO.BinaryWriter $stream
$w.Write([uint16]0)               # reserved
$w.Write([uint16]1)               # type: icon
$w.Write([uint16]$payloads.Count)
$offset = 6 + 16 * $payloads.Count
foreach ($p in $payloads) {
    $size = $p[0]; $bytes = $p[1]
    $dim = if ($size -ge 256) { 0 } else { $size }   # 0 means 256
    $w.Write([byte]$dim)          # width
    $w.Write([byte]$dim)          # height
    $w.Write([byte]0)             # colour count (0 = no palette)
    $w.Write([byte]0)             # reserved
    $w.Write([uint16]1)           # planes
    $w.Write([uint16]32)          # bits per pixel
    $w.Write([uint32]$bytes.Length) # bytes in resource
    $w.Write([uint32]$offset)     # offset of the image data
    $offset += $bytes.Length
}
# Cast matters: an Object[] of bytes would resolve to a one-byte overload.
foreach ($p in $payloads) { $w.Write([byte[]]$p[1]) }
$w.Flush(); $stream.Close()

Write-Svg $full (Join-Path $PSScriptRoot 'Weatherspell.svg')
Write-Svg $small24 (Join-Path $PSScriptRoot 'Weatherspell-24.svg')
Write-Svg $small16 (Join-Path $PSScriptRoot 'Weatherspell-16.svg')

"Wrote $out ($((Get-Item $out).Length) bytes, sizes: $(($frames | ForEach-Object { $_.Size }) -join ', ')) and the three SVGs"
