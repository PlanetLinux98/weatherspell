using Weatherspell.Weather.EnvironmentCanada;
using Weatherspell.Weather.Nws;
using Weatherspell.Weather.OpenMeteo;

namespace Weatherspell;

// Short on purpose: each source asks for a credit, which this gives, and the
// full wording, links and licences are in the guide's Credits and licences
// section, which the About dialog opens.
internal static class AboutText
{
    public static IReadOnlyList<string> Paragraphs(string version) =>
    [
        $"Weatherspell {version}\nA text-based weather app for Windows.",
        "Copyright 2026 PlanetLinux98. Released under the MIT licence.",
        $"Weather data from {OpenMeteoClient.SourceName} (CC BY 4.0), {CityPageClient.FullName} and the {NwsClient.SourceName}.",
        "Place names from GeoNames (CC BY 4.0) and OpenStreetMap contributors (ODbL).",
    ];
}
