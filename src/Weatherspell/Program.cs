namespace Weatherspell;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        // One copy per Windows session: two would each announce every alert
        // and save settings.json over each other's changes (#22). A second
        // launch brings the first one's window forward instead.
        using var instance = new Mutex(initiallyOwned: true, @"Local\Weatherspell", out var first);
        if (!first)
        {
            BringOtherCopyForward();
            return;
        }

        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new MainForm());
    }

    private static void BringOtherCopyForward()
    {
        using var self = System.Diagnostics.Process.GetCurrentProcess();
        foreach (var other in System.Diagnostics.Process.GetProcessesByName(self.ProcessName))
        {
            using (other)
            {
                var window = other.Id == self.Id ? IntPtr.Zero : other.MainWindowHandle;
                if (window == IntPtr.Zero) continue;
                if (Native.IsIconic(window)) Native.ShowWindow(window, Native.SW_RESTORE);
                Native.SetForegroundWindow(window);
                return;
            }
        }
    }
}
