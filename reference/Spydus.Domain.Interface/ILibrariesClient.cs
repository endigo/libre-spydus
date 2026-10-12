using System.Collections.Generic;
using System.Threading.Tasks;
using Refit;
using Spydus.Domain.DTO;

namespace Spydus.Domain.Interface;

public interface ILibrariesClient
{
	[Get("/mobile/libraries?groupCode={groupCode}")]
	Task<IApiResponse<List<LibraryDto>>> GetLibraries([AliasAs("groupCode")] string? groupCode);

	[Get("/mobile/languages?groupCode={groupCode}")]
	Task<IApiResponse<List<AppLanguageDto>>> GetLanguages([AliasAs("groupCode")] string? groupCode);
}
