using Weatherspell.Weather;
using Weatherspell.Weather.Nominatim;
using Xunit;

namespace Weatherspell.Tests;

public class CoordinatesTests
{
    [Theory]
    // Decimals, signed, with every separator people use.
    [InlineData("44.54, -78.54", 44.54, -78.54)]
    [InlineData("44.54,-78.54", 44.54, -78.54)]
    [InlineData("44.54 -78.54", 44.54, -78.54)]
    [InlineData("44.54;-78.54", 44.54, -78.54)]
    [InlineData("44.54 / -78.54", 44.54, -78.54)]
    [InlineData("  (44.54, -78.54)  ", 44.54, -78.54)]
    [InlineData("[44.54, -78.54]", 44.54, -78.54)]
    [InlineData("+44.54 -78.54", 44.54, -78.54)]
    [InlineData("44.54\t-78.54", 44.54, -78.54)]
    [InlineData("-33.87 151.21", -33.87, 151.21)]
    [InlineData("44, -78", 44, -78)]
    [InlineData("0, 0", 0, 0)]
    // Minus signs and dashes from word processors and web pages.
    [InlineData("44.54, \u221278.54", 44.54, -78.54)]
    [InlineData("44.54, \u201378.54", 44.54, -78.54)]
    // Compass letters and words, before or after, in either order.
    [InlineData("44.54 N 78.54 W", 44.54, -78.54)]
    [InlineData("44.54N 78.54W", 44.54, -78.54)]
    [InlineData("44.54N78.54W", 44.54, -78.54)]
    [InlineData("N44.54 W78.54", 44.54, -78.54)]
    [InlineData("N 44.54, W 78.54", 44.54, -78.54)]
    [InlineData("44.54 north, 78.54 west", 44.54, -78.54)]
    [InlineData("44.54 North 78.54 West", 44.54, -78.54)]
    [InlineData("44.54\u00B0 N, 78.54\u00B0 W", 44.54, -78.54)]
    [InlineData("44.54\u00B0N 78.54\u00B0W", 44.54, -78.54)]
    [InlineData("78.54 W 44.54 N", 44.54, -78.54)]
    [InlineData("W 78.54, N 44.54", 44.54, -78.54)]
    [InlineData("-78.54 W, 44.54 N", 44.54, -78.54)]
    [InlineData("33.87 S 151.21 E", -33.87, 151.21)]
    [InlineData("33.87 south, 151.21 east", -33.87, 151.21)]
    [InlineData("45.5 N 73.57 O", 45.5, -73.57)]
    [InlineData("45.5 nord, 73.57 ouest", 45.5, -73.57)]
    [InlineData("40.4 norte, 3.7 oeste", 40.4, -3.7)]
    // Degrees, minutes and seconds.
    [InlineData("44\u00B032'24\"N 78\u00B032'24\"W", 44.54, -78.54)]
    [InlineData("44\u00B032\u203224\u2033N 78\u00B032\u203224\u2033W", 44.54, -78.54)]
    [InlineData("44\u00B0 32\u2019 24\u201D N, 78\u00B0 32\u2019 24\u201D W", 44.54, -78.54)]
    [InlineData("44\u00BA32'24''N 78\u00BA32'24''W", 44.54, -78.54)]
    [InlineData("44\u00B0 32' 24\" N, 78\u00B0 32' 24\" W", 44.54, -78.54)]
    [InlineData("44 32 24 N 78 32 24 W", 44.54, -78.54)]
    [InlineData("N 44 32 24 W 78 32 24", 44.54, -78.54)]
    [InlineData("44:32:24 N 78:32:24 W", 44.54, -78.54)]
    [InlineData("44d 32m 24sec N, 78d 32m 24sec W", 44.54, -78.54)]
    [InlineData("44 degrees 32 minutes 24 seconds north, 78 degrees 32 minutes 24 seconds west", 44.54, -78.54)]
    [InlineData("44 32 24, -78 32 24", 44.54, -78.54)]
    [InlineData("44\u00B032'24\" -78\u00B032'24\"", 44.54, -78.54)]
    [InlineData("33\u00B052'12\"S 151\u00B012'36\"E", -33.87, 151.21)]
    // Degrees and decimal minutes, as GPS units and geocaching write them.
    [InlineData("N 44\u00B0 32.400 W 078\u00B0 32.400", 44.54, -78.54)]
    [InlineData("N44 32.4 W78 32.4", 44.54, -78.54)]
    [InlineData("44 32.4 N 78 32.4 W", 44.54, -78.54)]
    [InlineData("44 32.4, -78 32.4", 44.54, -78.54)]
    [InlineData("44\u00B0 32.4' -78\u00B0 32.4'", 44.54, -78.54)]
    // Decimal commas, as a French or German Windows writes numbers.
    [InlineData("44,54 -78,54", 44.54, -78.54)]
    [InlineData("44,54; -78,54", 44.54, -78.54)]
    [InlineData("44,54, -78,54", 44.54, -78.54)]
    [InlineData("44,54,-78,54", 44.54, -78.54)]
    [InlineData("45,5 N 73,57 O", 45.5, -73.57)]
    // Labels, in either order.
    [InlineData("lat 44.54 lon -78.54", 44.54, -78.54)]
    [InlineData("Latitude: 44.54, Longitude: -78.54", 44.54, -78.54)]
    [InlineData("lon -78.54 lat 44.54", 44.54, -78.54)]
    [InlineData("lat=44.54&lng=-78.54", 44.54, -78.54)]
    // Longitude first (GeoJSON), when the first number cannot be a latitude;
    // "-78.54, 44.54" is a real point in Antarctica, so it stays as typed.
    [InlineData("-93.27, 44.98", 44.98, -93.27)]
    [InlineData("-78.54, 44.54", -78.54, 44.54)]
    // Map links: the pin where there is one, else the view's centre.
    [InlineData("https://www.google.com/maps/place/Bobcaygeon/@44.54,-78.54,15z", 44.54, -78.54)]
    [InlineData("https://www.google.com/maps/place/X/@44.6,-78.6,15z/data=!3m1!4b1!4m6!3m5!1s0x0:0x0!8m2!3d44.54!4d-78.54", 44.54, -78.54)]
    [InlineData("https://maps.google.com/?q=44.54,-78.54", 44.54, -78.54)]
    [InlineData("https://www.google.com/maps/search/?api=1&query=44.54%2C-78.54", 44.54, -78.54)]
    [InlineData("maps.google.com/?q=44.54,-78.54", 44.54, -78.54)]
    [InlineData("https://www.openstreetmap.org/?mlat=44.54&mlon=-78.54#map=15/44.60/-78.60", 44.54, -78.54)]
    [InlineData("https://www.openstreetmap.org/#map=15/44.54/-78.54", 44.54, -78.54)]
    [InlineData("geo:44.54,-78.54", 44.54, -78.54)]
    [InlineData("geo:44.54,-78.54;u=35", 44.54, -78.54)]
    [InlineData("https://maps.apple.com/?ll=44.54,-78.54&q=Dropped%20Pin", 44.54, -78.54)]
    [InlineData("https://www.bing.com/maps?cp=44.54~-78.54&lvl=15", 44.54, -78.54)]
    public void Reads_a_point(string text, double latitude, double longitude)
    {
        var reading = Coordinates.Read(text);

        Assert.NotNull(reading);
        Assert.Null(reading!.Problem);
        Assert.Equal(latitude, reading.Latitude, 4);
        Assert.Equal(longitude, reading.Longitude, 4);
    }

    [Theory]
    [InlineData("Peterborough")]
    [InlineData("New York")]
    [InlineData("St. Catharines")]
    [InlineData("Paris 75004")]
    [InlineData("Route 66")]
    [InlineData("K9J 7B8")]
    [InlineData("SW1A 1AA")]
    [InlineData("M1")]
    [InlineData("N1")]
    [InlineData("M1 1AE")]
    [InlineData("E1 6AN")]
    [InlineData("D02 X285")]
    [InlineData("1011 AB")]
    [InlineData("2000")]
    [InlineData("90210")]
    [InlineData("12345-6789")]
    [InlineData("060-0001")]
    [InlineData("01310-100")]
    [InlineData("114 55")]
    [InlineData("110 00")]
    [InlineData("12345, 6789")]
    [InlineData("44")]
    [InlineData("44.54")]
    [InlineData("44.54 N")]
    [InlineData("")]
    [InlineData("   ")]
    public void Leaves_names_and_postal_codes_to_the_search(string text)
    {
        Assert.Null(Coordinates.Read(text));
    }

    [Theory]
    [InlineData("95.5 N, 78.54 W", "Couldn't use 95.5 as a latitude: it must be from 90 south to 90 north.")]
    [InlineData("100.5, 200.5", "Couldn't use 100.5 as a latitude: it must be from 90 south to 90 north.")]
    [InlineData("44.54, 200.5", "Couldn't use 200.5 as a longitude: it must be from 180 west to 180 east.")]
    [InlineData("44 65 N 78 32 W", "Couldn't read those coordinates: minutes and seconds must be under 60.")]
    [InlineData("44.54 N 78.54 N", "Couldn't read those coordinates: both are latitudes. Type a latitude and then a longitude, such as 44.54, -78.54.")]
    [InlineData("44.54 E 78.54 W", "Couldn't read those coordinates: both are longitudes. Type a latitude and then a longitude, such as 44.54, -78.54.")]
    [InlineData("44.54 -78.54 12.3", "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.")]
    [InlineData("-44.54 N, 78.54 W", "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.")]
    [InlineData("44.54, -78.54, 12.3", "Couldn't read those coordinates. Type a latitude and then a longitude, such as 44.54, -78.54.")]
    [InlineData("https://maps.app.goo.gl/AbCdEf123", "That link has no coordinates in it. Copy the coordinates themselves, or search for the place by name.")]
    [InlineData("https://www.google.com/maps/@95.5,-78.54,15z", "Couldn't use 95.5 as a latitude: it must be from 90 south to 90 north.")]
    public void Says_why_coordinates_cannot_be_used(string text, string problem)
    {
        var reading = Coordinates.Read(text);

        Assert.NotNull(reading);
        Assert.Equal(problem, reading!.Problem);
    }

    [Theory]
    [InlineData(44.54, -78.54, "44.54 north, 78.54 west")]
    [InlineData(-33.8688, 151.2093, "33.8688 south, 151.2093 east")]
    [InlineData(44.543219, -78.5, "44.5432 north, 78.5 west")]
    [InlineData(0, 0, "0 north, 0 east")]
    [InlineData(-0.00001, 0.00001, "0 north, 0 east")]
    public void Says_a_point_in_words(double latitude, double longitude, string words)
    {
        Assert.Equal(words, Coordinates.Words(latitude, longitude));
    }

    [Fact]
    public void Names_a_point_after_the_smallest_settlement_and_keeps_the_point_typed()
    {
        var place = NominatimClient.ParseReverse(Fixtures.Read("nominatim-bobcaygeon.json"), 44.54, -78.54);

        // The town, not the amalgamated city of Kawartha Lakes around it,
        // and the cottage's point, not the town's.
        Assert.NotNull(place);
        Assert.Equal("Bobcaygeon", place!.Name);
        Assert.Equal("Ontario", place.Region);
        Assert.Equal("Canada", place.Country);
        Assert.Equal(44.54, place.Latitude);
        Assert.Equal(-78.54, place.Longitude);
        Assert.Equal("Near Bobcaygeon, Ontario, Canada (44.54 north, 78.54 west)", place.NearText);
    }

    [Fact]
    public void Prefers_a_village_to_the_US_town_it_is_in()
    {
        var place = NominatimClient.ParseReverse(Fixtures.Read("nominatim-fort-hunter.json"), 42.75, -73.95);

        Assert.Equal("Fort Hunter, New York, United States", place!.FullName);
        Assert.True(place.IsNwsCovered);
    }

    [Fact]
    public void Calls_a_US_territory_by_its_name_as_the_geocoder_does()
    {
        var place = NominatimClient.ParseReverse(Fixtures.Read("nominatim-san-juan.json"), 18.4655, -66.1057);

        // The city, not the Viejo San Juan quarter Nominatim matched (#19).
        Assert.Equal("San Juan, Puerto Rico", place!.FullName);
        Assert.True(place.IsNwsCovered);
    }

    [Fact]
    public void Names_a_point_with_no_settlement_after_its_area()
    {
        var place = NominatimClient.ParseReverse(Fixtures.Read("nominatim-kitikmeot.json"), 70.0, -95.0);

        Assert.Equal("Kitikmeot Region, Nunavut, Canada", place!.FullName);
    }

    [Fact]
    public void A_point_nothing_names_goes_by_its_coordinates()
    {
        Assert.Null(NominatimClient.ParseReverse(Fixtures.Read("nominatim-atlantic.json"), 35.0, -40.0));

        var point = Location.AtPoint(35.0, -40.0);
        Assert.Equal("35 north, 40 west", point.FullName);
        Assert.Null(point.Country);
    }

    [Fact]
    public void Asks_for_the_point_in_invariant_numbers()
    {
        var url = NominatimClient.ReverseUrl(44.543219, -78.5);

        Assert.StartsWith("https://nominatim.openstreetmap.org/reverse?", url);
        Assert.EndsWith("&lat=44.543219&lon=-78.5", url);
    }
}
