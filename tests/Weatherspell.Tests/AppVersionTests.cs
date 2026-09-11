using Xunit;

namespace Weatherspell.Tests;

public class AppVersionTests
{
    [Fact]
    public void Display_is_a_bare_semver_string()
    {
        var display = AppVersion.Display;

        Assert.Matches(@"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$", display);
        Assert.DoesNotContain("+", display);
    }
}
