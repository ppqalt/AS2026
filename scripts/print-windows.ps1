param([Parameter(Mandatory=$true)][string]$Job, [string]$Printer = '', [string]$PreviewDirectory = '')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$jobData = [IO.File]::ReadAllText($Job, [Text.Encoding]::UTF8) | ConvertFrom-Json
function Draw-Page($graphics, $page) {
    $graphics.PageUnit = [Drawing.GraphicsUnit]::Millimeter
    $pen = [Drawing.Pen]::new([Drawing.Color]::Black, [single]0.15)
    $format = [Drawing.StringFormat]::new([Drawing.StringFormat]::GenericTypographic)
    $format.FormatFlags = [Drawing.StringFormatFlags]::NoWrap
    $format.Trimming = [Drawing.StringTrimming]::EllipsisCharacter
    try {
        foreach ($line in $page.lines) { $graphics.DrawLine($pen, [single]$line.x, [single]$line.y, [single]$line.x2, [single]$line.y2) }
        foreach ($text in $page.texts) {
            $style = if ($text.bold) { [Drawing.FontStyle]::Bold } else { [Drawing.FontStyle]::Regular }
            $font = [Drawing.Font]::new('Arial', [single]$text.size, $style, [Drawing.GraphicsUnit]::Point)
            try { $graphics.DrawString([string]$text.value, $font, [Drawing.Brushes]::Black, [Drawing.RectangleF]::new([single]$text.x, [single]$text.y, [single]$text.width, [single]$text.height), $format) }
            finally { $font.Dispose() }
        }
    } finally { $format.Dispose(); $pen.Dispose() }
}
if ($PreviewDirectory) {
    # Renderöintikoe: ei avaa tulostinta eikä lähetä tulostustyötä.
    [IO.Directory]::CreateDirectory($PreviewDirectory) | Out-Null
    $index = 0
    foreach ($page in $jobData.pages) {
        $width = if ($jobData.landscape) { 1754 } else { 1240 }
        $height = if ($jobData.landscape) { 1240 } else { 1754 }
        $bitmap = [Drawing.Bitmap]::new($width, $height)
        $bitmap.SetResolution(150,150)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try { $graphics.Clear([Drawing.Color]::White); Draw-Page $graphics $page; $bitmap.Save((Join-Path $PreviewDirectory ("sivu-{0}.png" -f (++$index))), [Drawing.Imaging.ImageFormat]::Png) }
        finally { $graphics.Dispose(); $bitmap.Dispose() }
    }
    exit 0
}
$document = [Drawing.Printing.PrintDocument]::new()
try {
    if ($Printer) { $document.PrinterSettings.PrinterName = $Printer }
    if (!$document.PrinterSettings.IsValid) { throw 'Valittua tulostinta ei ole käytettävissä. Valitse tulostin asetuksista.' }
    $document.DocumentName = $jobData.title
    $document.PrintController = [Drawing.Printing.StandardPrintController]::new()
    $document.DefaultPageSettings.Landscape = [bool]$jobData.landscape
    $a4 = @($document.PrinterSettings.PaperSizes | Where-Object { $_.Kind -eq [Drawing.Printing.PaperKind]::A4 })
    if ($a4.Count -eq 0) { throw 'Tulostin ei tue A4-paperia.' }
    $document.DefaultPageSettings.PaperSize = $a4[0]
    $script:pageIndex = 0
    $document.add_PrintPage({ param($sender,$eventArgs)
        # Mallin koordinaatit ovat paperin reunasta, GDI:n origo tulostusalueesta.
        $eventArgs.Graphics.PageUnit = [Drawing.GraphicsUnit]::Millimeter
        $eventArgs.Graphics.TranslateTransform(-$eventArgs.PageSettings.HardMarginX * 0.254, -$eventArgs.PageSettings.HardMarginY * 0.254)
        Draw-Page $eventArgs.Graphics $jobData.pages[$script:pageIndex]
        $script:pageIndex++
        $eventArgs.HasMorePages = $script:pageIndex -lt $jobData.pages.Count
    })
    $document.Print()
} finally { $document.Dispose() }
