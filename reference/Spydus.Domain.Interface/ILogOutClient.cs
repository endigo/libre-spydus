using System.Threading.Tasks;
using Refit;

namespace Spydus.Domain.Interface;

public interface ILogOutClient
{
	[Post("/openid/patronlogout")]
	Task<IApiResponse> LogOut([AliasAs("refreshToken")] string refreshToken, [AliasAs("clientId")] string clientId);
}
