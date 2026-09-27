using System.IO;
using System.Text;
using Xunit;

namespace Weatherspell.Tests;

public class UserGuideTests
{
    [Fact]
    public void The_guide_is_built_in_and_has_the_sections_the_app_opens()
    {
        var html = UserGuide.Html();

        Assert.StartsWith("<!DOCTYPE html>", html);
        Assert.Contains("<html lang=\"en-CA\">", html);
        Assert.Contains($"id=\"{UserGuide.Credits}\"", html);
    }

    [Fact]
    public void The_guide_opens_as_written_and_a_section_through_a_page_that_forwards_to_it()
    {
        var folder = Path.Combine(Path.GetTempPath(), "Weatherspell.Tests", Guid.NewGuid().ToString("N"));
        try
        {
            var guide = UserGuide.Prepare(folder);
            Assert.Equal(Path.Combine(folder, UserGuide.FileName), guide);
            Assert.Equal(UserGuide.Html(), File.ReadAllText(guide, Encoding.UTF8));

            var forward = UserGuide.Prepare(folder, UserGuide.Credits);
            Assert.Equal(Path.Combine(folder, "user-guide-credits-and-licences.html"), forward);
            Assert.Contains("<meta http-equiv=\"refresh\" content=\"0; url=user-guide.html#credits-and-licences\">", File.ReadAllText(forward));
        }
        finally
        {
            if (Directory.Exists(folder)) Directory.Delete(folder, recursive: true);
        }
    }

    [Fact]
    public void About_gives_the_version_and_credits_every_source()
    {
        var text = string.Join("\n\n", AboutText.Paragraphs("1.2.3"));

        Assert.StartsWith("Weatherspell 1.2.3\nA text-based weather app for Windows.", text);
        foreach (var source in new[] { "Open-Meteo", "Environment and Climate Change Canada", "National Weather Service", "GeoNames", "OpenStreetMap contributors" })
        {
            Assert.Contains(source, text);
        }
    }
}
