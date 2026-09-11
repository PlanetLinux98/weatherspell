using System.Runtime.Serialization.Json;
using System.Text;

namespace Weatherspell.Weather.OpenMeteo;

internal static class Json
{
    public static T Read<T>(string json) where T : class
    {
        var serializer = new DataContractJsonSerializer(typeof(T));
        using var stream = new MemoryStream(Encoding.UTF8.GetBytes(json));
        return (T)(serializer.ReadObject(stream) ?? throw new InvalidDataException("Empty JSON document."));
    }
}
