using System.Reflection;

namespace Weatherspell;

internal static class AppVersion
{
    // MinVer stamps AssemblyInformationalVersion from the git tag; the assembly
    // version alone loses the pre-release suffix. The "+sha" build metadata is
    // dropped for display.
    public static string Display { get; } =
        (typeof(AppVersion).Assembly
            .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
         ?? typeof(AppVersion).Assembly.GetName().Version?.ToString()
         ?? "0.0.0").Split('+')[0];
}
