namespace Weatherspell;

// Every form scales by font, not by DPI alone: the Windows "Text size"
// accessibility setting enlarges the system font without changing the
// display scale, and font-based scaling grows the layout with it so nothing
// clips. Sizes in the forms are written for Segoe UI 9 pt at 96 DPI, which
// is what AutoScaleDimensions declares.
internal static class Scaling
{
    public static void Apply(Form form)
    {
        // The system message font (Segoe UI on modern Windows) rather than
        // WinForms 4.8's default Microsoft Sans Serif 8.25 pt, so the app
        // follows the user's font and text-size settings like the rest of
        // Windows. Set before the scale mode so the first scale is measured
        // against the right font.
        form.Font = SystemFonts.MessageBoxFont ?? form.Font;
        form.AutoScaleDimensions = new SizeF(7F, 15F);
        form.AutoScaleMode = AutoScaleMode.Font;
    }
}
