namespace Weatherspell.Weather;

internal static class Compass
{
    private static readonly string[] Points =
    [
        "north", "north-northeast", "northeast", "east-northeast",
        "east", "east-southeast", "southeast", "south-southeast",
        "south", "south-southwest", "southwest", "west-southwest",
        "west", "west-northwest", "northwest", "north-northwest",
    ];

    // Sixteen points is the most anyone hears in a forecast; eight would
    // lose "north-northeast", which coastal listeners do use.
    public static string FromDegrees(double degrees)
    {
        var normalized = ((degrees % 360) + 360) % 360;
        var index = (int)Math.Round(normalized / 22.5, MidpointRounding.AwayFromZero) % 16;
        return Points[index];
    }
}
