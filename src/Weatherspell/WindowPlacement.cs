using Weatherspell.Settings;

namespace Weatherspell;

// The main window opens where it was closed, if that is still somewhere a
// user can reach it. Screens come and go (a laptop off its dock, a remote
// session at another resolution), so the saved place is checked against
// the working areas there are now rather than trusted.
internal static class WindowPlacement
{
    // charSize is the form's CurrentAutoScaleDimensions: its font's average
    // character, which is what Scaling sizes every form by.
    public static SavedWindow Save(Rectangle normalBounds, bool maximized, SizeF charSize) => new()
    {
        Left = normalBounds.Left,
        Top = normalBounds.Top,
        Width = normalBounds.Width,
        Height = normalBounds.Height,
        Maximized = maximized,
        CharWidth = Math.Round(charSize.Width, 2),
        CharHeight = Math.Round(charSize.Height, 2),
    };

    // The normal bounds to open with, or null to open centred as on a first
    // run: when no screen shows enough of the title bar to take hold of.
    // The size follows a change of display scale or Text size since it was
    // saved, as the window's text does (across and down separately: a font
    // does not grow in proportion), and the window is kept on the screen it
    // lands on, give or take the invisible resize border Windows counts in
    // a window's bounds (a snapped window reaches past the edge).
    public static Rectangle? Restore(SavedWindow saved, SizeF charSize, IReadOnlyList<Rectangle> workingAreas, int captionHeight)
    {
        if (saved.Width <= 0 || saved.Height <= 0) return null;
        var width = Follow(saved.Width, saved.CharWidth, charSize.Width);
        var height = Follow(saved.Height, saved.CharHeight, charSize.Height);

        var titleBar = new Rectangle(saved.Left, saved.Top, width, captionHeight);
        var reachable = workingAreas
            .Select(area => (area, shown: Rectangle.Intersect(area, titleBar)))
            .Where(a => a.shown.Width >= Math.Min(MinimumGrip, width) && a.shown.Height >= captionHeight / 2)
            .OrderByDescending(a => a.shown.Width * a.shown.Height)
            .Select(a => (Rectangle?)a.area)
            .FirstOrDefault();
        if (reachable is not Rectangle screen) return null;

        // The invisible border is on the sides and the bottom (the top is
        // title bar), about half a caption high at any scale.
        var border = captionHeight / 2;
        var (left, fittedWidth) = Fit(saved.Left, width, screen.Left, screen.Right, border, border);
        var (top, fittedHeight) = Fit(saved.Top, height, screen.Top, screen.Bottom, 0, border);
        return new Rectangle(left, top, fittedWidth, fittedHeight);
    }

    // Enough of the title bar to drag, in pixels.
    private const int MinimumGrip = 100;

    // A file from before the character size was kept, or edited by hand,
    // has none: the size is taken as it is.
    private static int Follow(int length, double savedChar, float currentChar) =>
        savedChar > 0 && currentChar > 0 ? (int)Math.Round(length * currentChar / savedChar) : length;

    // A span within [low, high], give or take the border, stays as it is;
    // one that overhangs further is moved wholly inside, shortened to fit.
    private static (int Start, int Length) Fit(int start, int length, int low, int high, int borderLow, int borderHigh)
    {
        if (start >= low - borderLow && start + length <= high + borderHigh) return (start, length);
        length = Math.Min(length, high - low);
        return (Math.Max(low, Math.Min(start, high - length)), length);
    }
}
