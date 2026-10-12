using System.Threading;
using System.Threading.Tasks;
using Refit;

namespace Spydus.Domain.Interface;

public interface ILogoClient
{
	[Get("/api/mobile/1.0/general/logo")]
	Task<IApiResponse<string>> GetLogo(CancellationToken cancellationToken);
}
