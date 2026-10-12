using System.Threading.Tasks;
using Refit;

namespace Spydus.Domain.Interface;

public interface IBookClient
{
	[Get("/api/mobile/1.0/discover/search?{searchMode}={searchData}")]
	Task<string> GetSearchBook(string? searchMode, string? searchData, [Header("spydus-lang")] string? languageIdentifier);
}
