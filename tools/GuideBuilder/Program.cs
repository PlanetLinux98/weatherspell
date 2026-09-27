using System.Text;
using Markdig;
using Markdig.Extensions.AutoIdentifiers;

// USER_GUIDE.md is the guide as GitHub shows it; this puts the same text in
// the page the exe opens on F1 (see src\Weatherspell\UserGuide.cs).
if (args.Length != 3)
{
    Console.Error.WriteLine("Usage: GuideBuilder <guide.md> <template.html> <out.html>");
    return 2;
}

var pipeline = new MarkdownPipelineBuilder()
    .UsePipeTables()
    // GitHub's heading ids, so a link such as #credits-and-licences works
    // on GitHub and in the built page alike.
    .UseAutoIdentifiers(AutoIdentifierOptions.GitHub)
    .Build();

var body = Markdown.ToHtml(File.ReadAllText(args[0], Encoding.UTF8), pipeline).TrimEnd();
var page = File.ReadAllText(args[1], Encoding.UTF8).Replace("{{content}}", body);
Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(args[2]))!);
File.WriteAllText(args[2], page, new UTF8Encoding(false));
return 0;
