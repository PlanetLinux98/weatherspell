using System.Net.Http;
using System.Net.Http.Headers;

namespace Weatherspell;

// One HttpClient for the process: reused sockets, one User-Agent (the NWS
// refuses anonymous requests and the others ask for identification), one
// timeout. Never blocks the UI thread; callers await.
internal static class Http
{
    public static readonly HttpClient Client = Create();

    private static HttpClient Create()
    {
        var client = new HttpClient { Timeout = TimeSpan.FromSeconds(20) };
        client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("Weatherspell", AppVersion.Display));
        client.DefaultRequestHeaders.UserAgent.Add(new ProductInfoHeaderValue("(+https://github.com/PlanetLinux98/weatherspell)"));
        client.DefaultRequestHeaders.Accept.Add(new MediaTypeWithQualityHeaderValue("application/json"));
        return client;
    }

    public static async Task<string> GetStringAsync(string url, CancellationToken cancellationToken)
    {
        using var response = await Client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, cancellationToken).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
        {
            throw new HttpRequestException($"{(int)response.StatusCode} {response.ReasonPhrase} from {new Uri(url).Host}");
        }
        return await response.Content.ReadAsStringAsync().ConfigureAwait(false);
    }
}
