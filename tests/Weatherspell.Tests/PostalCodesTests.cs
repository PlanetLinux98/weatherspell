using Weatherspell.Weather;
using Xunit;

namespace Weatherspell.Tests;

public class PostalCodesTests
{
    [Theory]
    [InlineData("K9J", "K9J")]
    [InlineData("k9j 7b8", "K9J")]
    [InlineData("K9J7B8", "K9J")]
    [InlineData("  K9J 7  ", "K9J")]
    [InlineData("M1", "M1")]
    [InlineData("m1 1ae", "M1")]
    [InlineData("M11AE", "M1")]
    [InlineData("SW1A 1AA", "SW1A")]
    [InlineData("sw1a1aa", "SW1A")]
    [InlineData("D02 X285", "D02")]
    [InlineData("D02X285", "D02")]
    [InlineData("2000", "2000")]
    public void Reads_the_part_of_a_code_the_table_keys_on(string query, string key)
    {
        Assert.Contains(key, PostalCodes.Keys(query));
    }

    [Theory]
    [InlineData("Peterborough")]
    [InlineData("New York")]
    [InlineData("49620")]
    [InlineData("Dublin 2")]
    [InlineData("")]
    public void Finds_nothing_for_place_names_and_codes_of_other_countries(string query)
    {
        Assert.Empty(PostalCodes.Find(query));
    }

    [Fact]
    public void A_full_Canadian_code_resolves_to_its_forward_sortation_area()
    {
        var place = Assert.Single(PostalCodes.Find("K9J 7B8"));

        Assert.Equal("Peterborough South", place.Name);
        Assert.Equal("Ontario", place.Region);
        Assert.Equal("Canada", place.Country);
        Assert.Equal("Peterborough South, Ontario, Canada", place.SearchResultText);
        Assert.Equal(44.3104, place.Latitude, 4);
        Assert.Equal(-78.2396, place.Longitude, 4);
    }

    [Fact]
    public void A_code_shared_by_several_places_is_one_entry_named_after_the_largest()
    {
        // L9X covers Barrie and three townships; E17 lists a tube station
        // before Walthamstow; IP8 is thirteen Suffolk villages.
        Assert.Equal("Barrie, Ontario, Canada", Assert.Single(PostalCodes.Find("L9X")).FullName);
        Assert.Equal("Walthamstow, England, United Kingdom", Assert.Single(PostalCodes.Find("E17")).FullName);
        Assert.Equal("Bramford, England, United Kingdom", Assert.Single(PostalCodes.Find("IP8")).FullName);
        Assert.Equal("Sydney, New South Wales, Australia", Assert.Single(PostalCodes.Find("2000")).FullName);
    }

    [Fact]
    public void A_code_used_in_two_countries_lists_both()
    {
        var found = PostalCodes.Find("1010");

        Assert.Equal(["Sydney, New South Wales, Australia", "Auckland, New Zealand"], found.Select(p => p.FullName));
    }

    [Theory]
    [InlineData("M1 1AE", "Manchester, England, United Kingdom")]
    [InlineData("EH1", "Edinburgh, Scotland, United Kingdom")]
    [InlineData("BT62", "Craigavon, Northern Ireland, United Kingdom")]
    [InlineData("3000", "Melbourne, Victoria, Australia")]
    [InlineData("6011", "Mount Victoria, Wellington, New Zealand")]
    [InlineData("D02 X285", "Dublin 2, Leinster, Ireland")]
    [InlineData("T12", "Cork city southside, Munster, Ireland")]
    [InlineData("H0H 0H0", "Reserved (Santa Claus), Quebec, Canada")]
    public void Finds_codes_in_each_covered_country(string query, string fullName)
    {
        Assert.Contains(fullName, PostalCodes.Find(query).Select(p => p.FullName));
    }

    [Fact]
    public void Territories_are_named_as_the_geocoder_names_them()
    {
        Assert.Equal("Northwest Territories", Assert.Single(PostalCodes.Find("X1A")).Region);
        Assert.Equal("Nunavut", Assert.Single(PostalCodes.Find("X0A")).Region);
    }

    [Fact]
    public void A_region_the_data_leaves_blank_is_left_out_of_the_name()
    {
        Assert.Equal("Jersey, United Kingdom", Assert.Single(PostalCodes.Find("JE4")).FullName);
    }

    [Fact]
    public void The_embedded_table_parses_completely_and_plausibly()
    {
        // The exe's embedded copy, read the way the app reads it, so an
        // accented name or a regenerated file with a bad row shows up here.
        using var stream = typeof(PostalCodes).Assembly.GetManifestResourceStream("PostalCodes.tsv")!;
        using var reader = new StreamReader(stream);
        var rows = 0;
        var countries = new HashSet<string>();
        while (reader.ReadLine() is { } line)
        {
            if (line.Length == 0 || line[0] == '#') continue;
            rows++;
            var code = line.Substring(0, line.IndexOf('\t'));
            var found = PostalCodes.Find(code);
            Assert.NotEmpty(found);
            foreach (var p in found)
            {
                countries.Add(p.Country!);
                Assert.False(string.IsNullOrWhiteSpace(p.Name));
                Assert.Contains(p.Country, PostalCodes.Countries);
                if (code == "H0H") continue; // Santa Claus, at the North Pole
                Assert.InRange(p.Latitude, -55, 84);
                Assert.InRange(p.Longitude, -142, 180);
            }
        }
        Assert.InRange(rows, 9000, 12000);
        Assert.Equal(PostalCodes.Countries.OrderBy(c => c), countries.OrderBy(c => c));
        Assert.Equal("Lamèque", Assert.Single(PostalCodes.Find("E8T")).Name);
    }
}

public class LocationSearchTests
{
    private static Location At(string name, string? region, string? country) => new(name, region, country, 0, 0, null);

    [Fact]
    public void Table_entries_come_first_and_replace_the_geocoder_for_their_countries()
    {
        var table = new[] { At("Sydney", "New South Wales", "Australia") };
        var geocoder = new[]
        {
            At("Antwerpen", "Flanders", "Belgium"),
            At("Somewhere", "Queensland", "Australia"),
            At("Elsewhere", null, country: null),
        };

        var merged = LocationSearch.Merge(table, geocoder);

        Assert.Equal(["Sydney", "Antwerpen", "Elsewhere"], merged.Select(l => l.Name));
    }

    [Fact]
    public void Without_table_entries_the_geocoder_answer_is_untouched()
    {
        // A place-name search must still find places in covered countries.
        var geocoder = new[] { At("Peterborough", "Ontario", "Canada") };

        Assert.Same(geocoder, LocationSearch.Merge([], geocoder));
    }
}
