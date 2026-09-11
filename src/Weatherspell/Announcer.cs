using System.Windows.Forms.Automation;

namespace Weatherspell;

// Speaks a line through UI Automation without moving focus: how search
// results counts and, later, new alerts reach a screen reader user.
// Available from .NET Framework 4.7.3; silently a no-op when the control has
// no handle yet, since the notification rides on the window's provider.
internal static class Announcer
{
    public static void Say(Control control, string text)
    {
        if (!control.IsHandleCreated) return;
        control.AccessibilityObject.RaiseAutomationNotification(
            AutomationNotificationKind.ActionCompleted,
            AutomationNotificationProcessing.MostRecent,
            text);
    }
}
