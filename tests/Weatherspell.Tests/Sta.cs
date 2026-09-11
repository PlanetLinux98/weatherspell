using System.Runtime.ExceptionServices;

namespace Weatherspell.Tests;

internal static class Sta
{
    // WinForms controls must be created on an STA thread; xUnit runs tests on
    // MTA threads. No handle is ever created, so this works on a headless CI
    // runner too.
    public static T Run<T>(Func<T> body)
    {
        T result = default!;
        Exception? error = null;
        var thread = new Thread(() =>
        {
            try { result = body(); }
            catch (Exception ex) { error = ex; }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();
        thread.Join();
        if (error is not null)
        {
            ExceptionDispatchInfo.Capture(error).Throw();
        }
        return result;
    }
}
