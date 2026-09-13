using System.Runtime.Serialization.Json;
using System.Text;

namespace Weatherspell.Weather;

// Shared by every JSON source (Open-Meteo, the NWS, GeoMet): the in-box
// DataContractJsonSerializer, since System.Text.Json would ship a DLL. The
// simple dictionary format reads a JSON object into a Dictionary (the NWS
// alert "parameters" map); the default would expect key/value pair arrays.
internal static class Json
{
    private static readonly DataContractJsonSerializerSettings Settings = new() { UseSimpleDictionaryFormat = true };

    public static T Read<T>(string json) where T : class
    {
        var serializer = new DataContractJsonSerializer(typeof(T), Settings);
        using var stream = new MemoryStream(Encoding.UTF8.GetBytes(json));
        return (T)(serializer.ReadObject(stream) ?? throw new InvalidDataException("Empty JSON document."));
    }
}
