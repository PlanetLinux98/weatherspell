namespace Weatherspell;

// Every form scales by font, not by DPI alone: the Windows "Text size"
// accessibility setting enlarges the system font without changing the
// display scale, and font-based scaling grows the layout with it so nothing
// clips. Sizes in the forms are written for Segoe UI 9 pt at 96 DPI, which
// is what AutoScaleDimensions declares.
internal static class Scaling
{
    // Segoe UI 9 pt at 96 DPI measures 7 x 15; every size in the forms is
    // written in those units.
    private static readonly SizeF DesignDimensions = new(7F, 15F);

    // Call AFTER the form's Size and MinimumSize are set and BEFORE controls
    // are added. WinForms scales the window itself only for sizes assigned
    // before AutoScaleDimensions and AutoScaleMode; anything set afterwards
    // is never scaled (measured: 520x420 set first becomes 733x663 at 144
    // DPI, set after it stays 520x420). Children are scaled at first layout.
    public static void Apply(Form form)
    {
        // The system message font (Segoe UI on modern Windows) rather than
        // WinForms 4.8's default Microsoft Sans Serif 8.25 pt, so the app
        // follows the user's font and text-size settings like the rest of
        // Windows. Set before the scale mode so the first scale is measured
        // against the right font.
        form.Font = DeveloperFont() ?? SystemFonts.MessageBoxFont ?? form.Font;
        form.AutoScaleDimensions = DesignDimensions;
        form.AutoScaleMode = AutoScaleMode.Font;

        // A large font on a small screen can scale a window past the screen
        // edge, taking its buttons with it; WinForms does not clamp.
        var area = Screen.PrimaryScreen.WorkingArea;
        form.MinimumSize = new Size(Math.Min(form.MinimumSize.Width, area.Width), Math.Min(form.MinimumSize.Height, area.Height));
        form.Size = new Size(Math.Min(form.Width, area.Width), Math.Min(form.Height, area.Height));
    }

    // WEATHERSPELL_FONT_POINTS=18 makes every form start with a large font,
    // as a user's Text size setting would, so scaling can be checked on a
    // machine that has it at the default (tools/Show-Scaled.ps1). Unset in
    // normal use.
    private static Font? DeveloperFont()
    {
        var value = Environment.GetEnvironmentVariable("WEATHERSPELL_FONT_POINTS");
        if (string.IsNullOrEmpty(value)) return null;
        return float.TryParse(value, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out var points) && points is >= 6 and <= 72
            ? new Font((SystemFonts.MessageBoxFont ?? SystemFonts.DefaultFont).FontFamily, points)
            : null;
    }
}
