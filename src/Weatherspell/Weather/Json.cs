using System.Runtime.Serialization.Json;
using System.Text;

namespace Weatherspell.Weather;

// Shared by every JSON source (Open-Meteo, the NWS): the in-box
// DataContractJsonSerializer, since System.Text.Json would ship a DLL.
internal static class Json
{
    public static T Read<T>(string json) where T : class
    {
        var serializer = new DataContractJsonSerializer(typeof(T));
        using var stream = new MemoryStream(Encoding.UTF8.GetBytes(json));
        return (T)(serializer.ReadObject(stream) ?? throw new InvalidDataException("Empty JSON document."));
    }
}
